extern "C" {
#include "alvr_streamer.h"
}
#include "bindings.h"
#include "chaperone.h"
#include "controller.h"
#include "generic_tracker.h"
#include "hmd.h"
#include "tracked_devices.h"
#include <map>
#include <optional>
#include <string>
#include <thread>
#include <vector>

class DriverProvider : vr::IServerTrackedDeviceProvider {
  public:
    Hmd hmd;
    std::optional<Controller> left_controller, right_controller;
    std::vector<GenericTracker> generic_trackers;

    std::map<uint64_t, TrackedDevice *> tracked_devices;

    std::optional<std::thread> event_thread;
    bool running = false;

    void event_loop() {
        // set_chaperone({1.0, 1.0});

        while (this->running) {
            auto event = alvr_read_event(100); // ms

            if (event.ty == ALVR_EVENT_TYPE_DEVICE_CONNECTED) {
                auto profile = event.data.device_profile;

                auto device_it = this->tracked_devices.find(profile.top_level_path);
                if (device_it == this->tracked_devices.end()) {
                    if (profile.top_level_path == HEAD_PATH) {
                        // unreachable
                    } else if (profile.top_level_path == LEFT_HAND_PATH) {
                        this->left_controller =
                            Controller(LEFT_HAND_PATH, profile.interaction_profile);
                        this->tracked_devices.insert({LEFT_HAND_PATH, &*this->left_controller});
                    } else if (profile.top_level_path == RIGHT_HAND_PATH) {
                        this->right_controller =
                            Controller(RIGHT_HAND_PATH, profile.interaction_profile);
                        this->tracked_devices.insert({RIGHT_HAND_PATH, &*this->right_controller});
                    } else {
                        this->generic_trackers.push_back(GenericTracker(profile.top_level_path));
                        this->tracked_devices.insert(
                            {profile.top_level_path,
                             &this->generic_trackers[this->generic_trackers.size() - 1]});
                    }
                } else {
                    vr::VRServerDriverHost()->VendorSpecificEvent(
                        device_it->second->object_id, vr::VREvent_WirelessReconnect, {}, 0);
                }
            } else if (event.ty == ALVR_EVENT_TYPE_DEVICE_DISCONNECTED) {
                auto device_it = this->tracked_devices.find(event.data.top_level_path);
                if (device_it != this->tracked_devices.end()) {
                    vr::VRServerDriverHost()->VendorSpecificEvent(
                        device_it->second->object_id, vr::VREvent_WirelessDisconnect, {}, 0);
                    device_it->second->clear_pose();
                }
            } else if (event.ty == ALVR_EVENT_TYPE_OPENVR_PROPERTY_CHANGED) {
                auto device_it = this->tracked_devices.find(event.data.openvr_prop.top_level_path);
                if (device_it != this->tracked_devices.end()) {
                    device_it->second->set_prop(event.data.openvr_prop.prop);
                }
            } else if (event.ty == ALVR_EVENT_TYPE_VIDEO_CONFIG_UPDATED) {
                this->hmd.update_video_config(event.data.video_config);
            } else if (event.ty == ALVR_EVENT_TYPE_VIEWS_CONFIG_UPDATED) {
                this->hmd.update_views_config(event.data.views_config);
            } else if (event.ty == ALVR_EVENT_TYPE_DEVICE_POSE_UPDATED) {
                auto event_data = event.data.device_pose;

                auto device_it = this->tracked_devices.find(event_data.top_level_path);
                if (device_it != this->tracked_devices.end()) {
                    device_it->second->update_pose(event_data.data, event_data.timestamp_ns);
                }
            } else if (event.ty == ALVR_EVENT_TYPE_BUTTON_UPDATED) {
                this->left_controller->try_update_button(event.data.button);
                this->right_controller->try_update_button(event.data.button);
            } else if (event.ty == ALVR_EVENT_TYPE_HAND_SKELETON_UPDATED) {
                if (event.data.hand_skeleton.hand_type == ALVR_HAND_TYPE_LEFT) {
                    this->left_controller->update_hand_skeleton(
                        event.data.hand_skeleton.joints, event.data.hand_skeleton.timestamp_ns);
                } else {
                    this->right_controller->update_hand_skeleton(
                        event.data.hand_skeleton.joints, event.data.hand_skeleton.timestamp_ns);
                }
            } else if (event.ty == ALVR_EVENT_TYPE_BATTERY_UPDATED) {
                auto device_it = this->tracked_devices.find(event.data.battery.top_level_path);
                if (device_it != this->tracked_devices.end()) {
                    vr::VRProperties()->SetFloatProperty(device_it->second->object_id,
                                                         vr::Prop_DeviceBatteryPercentage_Float,
                                                         event.data.battery.value);
                }
            } else if (event.ty == ALVR_EVENT_TYPE_BOUNDS_UPDATED) {
                set_chaperone(event.data.bounds_rect);
            } else if (event.ty == ALVR_EVENT_TYPE_RESTART_REQUESTED) {
                vr::VRServerDriverHost()->RequestRestart(
                    "ALVR requested SteamVR restart", "", "", "");
            } else if (event.ty == ALVR_EVENT_TYPE_SHUTDOWN_REQUESTED) {
                vr::VRServerDriverHost()->VendorSpecificEvent(
                    0, vr::VREvent_DriverRequestedQuit, {}, 0);
            }
        }
    }

    virtual vr::EVRInitError Init(vr::IVRDriverContext *context) override {
        VR_INIT_SERVER_DRIVER_CONTEXT(context);

        auto graphics_context = AlvrGraphicsContext{};
#ifdef _WIN32
        graphics_context.vk_get_device_proc_addr = nullptr;
#else
        // todo: initialize from vulkan layer
#endif

        if (alvr_initialize(graphics_context)) {
            this->tracked_devices.insert({HEAD_PATH, &this->hmd});

            char hmd_serial_number[64];
            alvr_get_serial_number(HEAD_PATH, hmd_serial_number, 64);

            // If there is another HMD connected this call will fail. ALVR will continue using the
            // Hmd instance, but its data will remain unused.
            vr::VRServerDriverHost()->TrackedDeviceAdded(
                hmd_serial_number, vr::TrackedDeviceClass_HMD, &this->hmd);

            this->running = true;
            // this->event_thread = std::thread(&DriverProvider::event_loop, this);

            return vr::VRInitError_None;
        } else {
            return vr::VRInitError_Driver_Failed;
        }
    }

    virtual void Cleanup() override {
        alvr_popup_error("cleanup");

        running = false;
        if (event_thread) {
            event_thread->join();
        }

        alvr_shutdown();

        VR_CLEANUP_SERVER_DRIVER_CONTEXT();
    }

    virtual const char *const *GetInterfaceVersions() override { return vr::k_InterfaceVersions; }

    virtual const char *GetTrackedDeviceDriverVersion() {
        return vr::ITrackedDeviceServerDriver_Version;
    }

    virtual void RunFrame() override {
        // vr::VRServerDriverHost()->VsyncEvent(0.016);

        // vr::VREvent_t event;
        // while (vr::VRServerDriverHost()->PollNextEvent(&event, sizeof(vr::VREvent_t))) {
        //     if (event.eventType == vr::VREvent_Input_HapticVibration) {
        //         vr::VREvent_HapticVibration_t haptics_info = event.data.hapticVibration;

        //         if (this->left_controller &&
        //             haptics_info.containerHandle == this->left_controller->haptics_container) {
        //             alvr_send_haptics(LEFT_HAND_PATH,
        //                               haptics_info.fDurationSeconds,
        //                               haptics_info.fFrequency,
        //                               haptics_info.fAmplitude);
        //         } else if (this->right_controller &&
        //                    haptics_info.containerHandle ==
        //                        this->right_controller->haptics_container) {
        //             alvr_send_haptics(RIGHT_HAND_PATH,
        //                               haptics_info.fDurationSeconds,
        //                               haptics_info.fFrequency,
        //                               haptics_info.fAmplitude);
        //         }
        //     }
        // }
    }

    virtual bool ShouldBlockStandbyMode() override { return false; }

    virtual void EnterStandby() override {}

    virtual void LeaveStandby() override {}

    DriverProvider() {}
} g_driver_provider;

void *entry_point(const char *interface_name, int *return_code) {
    if (std::string(interface_name) == vr::IServerTrackedDeviceProvider_Version) {
        *return_code = vr::VRInitError_None;
        return &g_driver_provider;
    } else {
        *return_code = vr::VRInitError_Init_InterfaceNotFound;
        return nullptr;
    }
}