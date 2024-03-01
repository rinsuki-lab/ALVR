use crate::GraphicsContext;
use alvr_common::glam::UVec2;
use glow as gl;
use glow::HasContext;
use hal::{MemoryFlags, TextureUses};
use khronos_egl as egl;
use std::{mem, num::NonZeroU32, os::raw::c_void, ptr, rc::Rc, sync::Arc};
use wgpu::{
    hal, Device, Instance, InstanceDescriptor, InstanceFlags, Texture, TextureDescriptor,
    TextureUsages,
};
use wgpu_core::api;

pub const GL_TEXTURE_EXTERNAL_OES: u32 = 0x8D65;
const EGL_NATIVE_BUFFER_ANDROID: u32 = 0x3140;

const CREATE_IMAGE_FN_STR: &str = "eglCreateImageKHR";
const DESTROY_IMAGE_FN_STR: &str = "eglDestroyImageKHR";
const GET_NATIVE_CLIENT_BUFFER_FN_STR: &str = "eglGetNativeClientBufferANDROID";
const IMAGE_TARGET_TEXTURE_2D_FN_STR: &str = "glEGLImageTargetTexture2DOES";

type CreateImageFn = unsafe extern "C" fn(
    egl::EGLDisplay,
    egl::EGLContext,
    egl::Enum,
    egl::EGLClientBuffer,
    *const egl::Int,
) -> egl::EGLImage;
type DestroyImageFn = unsafe extern "C" fn(egl::EGLDisplay, egl::EGLImage) -> egl::Boolean;
type GetNativeClientBufferFn = unsafe extern "C" fn(*const c_void) -> egl::EGLClientBuffer;
type ImageTargetTexture2DFn = unsafe extern "C" fn(egl::Enum, egl::EGLImage);

#[derive(Clone)]
pub struct GlBackend {
    pub egl_display: egl::Display,
    pub egl_config: egl::Config,
    pub egl_context: egl::Context,
    pub gl_context: Rc<gl::Context>,
    create_image: CreateImageFn,
    destroy_image: DestroyImageFn,
    get_native_client_buffer: GetNativeClientBufferFn,
    image_target_texture_2d: ImageTargetTexture2DFn,
}

impl GraphicsContext<GlBackend> {
    fn get_fn_ptr(adapter: &wgpu::Adapter, name: &str) -> *const c_void {
        unsafe {
            adapter.as_hal::<api::Gles, _, _>(|a| {
                let egl = a.unwrap().adapter_context().egl_instance().unwrap();
                egl.get_proc_address(name).unwrap() as *const c_void
            })
        }
    }

    pub fn new_gles() -> Self {
        let flags = if cfg!(debug_assertions) {
            InstanceFlags::DEBUG | InstanceFlags::VALIDATION
        } else {
            InstanceFlags::empty()
        };

        let instance = Instance::new(InstanceDescriptor {
            backends: wgpu::Backends::GL,
            flags,
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
        });
        let adapter = instance.enumerate_adapters(wgpu::Backends::GL).remove(0);
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .unwrap();

        let raw_instance = unsafe { instance.as_hal::<api::Gles>() }.unwrap();

        let egl_display = raw_instance.raw_display();
        let egl_config = raw_instance.egl_config();

        let (egl_context, gl_context) = unsafe {
            adapter.as_hal::<api::Gles, _, _>(|raw_adapter| {
                let adapter_context = raw_adapter.unwrap().adapter_context();
                let egl_context = egl::Context::from_ptr(adapter_context.raw_context());
                let gl_context = gl::Context::from_loader_function(|s| {
                    adapter_context
                        .egl_instance()
                        .unwrap()
                        .get_proc_address(s)
                        .unwrap() as *const _
                });

                (egl_context, Rc::new(gl_context))
            })
        };

        let create_image =
            unsafe { mem::transmute(Self::get_fn_ptr(&adapter, CREATE_IMAGE_FN_STR)) };
        let destroy_image =
            unsafe { mem::transmute(Self::get_fn_ptr(&adapter, DESTROY_IMAGE_FN_STR)) };
        let get_native_client_buffer =
            unsafe { mem::transmute(Self::get_fn_ptr(&adapter, GET_NATIVE_CLIENT_BUFFER_FN_STR)) };
        let image_target_texture_2d =
            unsafe { mem::transmute(Self::get_fn_ptr(&adapter, IMAGE_TARGET_TEXTURE_2D_FN_STR)) };

        Self {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            backend_handles: GlBackend {
                egl_display,
                egl_config,
                egl_context,
                gl_context,
                create_image,
                destroy_image,
                get_native_client_buffer,
                image_target_texture_2d,
            },
        }
    }

    /// # Safety
    /// `buffer` must be a valid AHardwareBuffer.
    /// `texture` must be a valid GL texture.
    pub unsafe fn bind_ahardwarebuffer_to_gl_ext_texture(
        &self,
        buffer: *const c_void,
        texture: gl::Texture,
    ) -> egl::EGLImage {
        let client_buffer = (self.backend_handles.get_native_client_buffer)(buffer);

        let image = (self.backend_handles.create_image)(
            self.backend_handles.egl_display.as_ptr(),
            egl::NO_CONTEXT,
            EGL_NATIVE_BUFFER_ANDROID,
            client_buffer,
            ptr::null(),
        );

        self.backend_handles
            .gl_context
            .bind_texture(GL_TEXTURE_EXTERNAL_OES, Some(texture));

        (self.backend_handles.image_target_texture_2d)(GL_TEXTURE_EXTERNAL_OES, image);

        image
    }

    /// # Safety
    /// `image` must be a valid EGLImage.
    pub unsafe fn destroy_image(&self, image: egl::EGLImage) {
        (self.backend_handles.destroy_image)(self.backend_handles.egl_display.as_ptr(), image);
    }
}

// This is used to convert OpenXR swapchains to wgpu
// textures should be arrays of depth 2, RGBA8UnormSrgb
pub fn create_texture_from_gles(device: &Device, texture: u32, resolution: UVec2) -> Texture {
    unsafe {
        let hal_texture = device
            .as_hal::<api::Gles, _, _>(|device| {
                device.unwrap().texture_from_raw_renderbuffer(
                    NonZeroU32::new(texture).unwrap(),
                    &hal::TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d {
                            width: resolution.x,
                            height: resolution.y,
                            depth_or_array_layers: 2,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: TextureUses::COLOR_TARGET,
                        memory_flags: MemoryFlags::empty(),
                        view_formats: vec![],
                    },
                    Some(Box::new(())),
                )
            })
            .unwrap();

        device.create_texture_from_hal::<api::Gles>(
            hal_texture,
            &TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: resolution.x,
                    height: resolution.y,
                    depth_or_array_layers: 2,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        )
    }
}

pub fn create_gl_swapchain(device: &Device, textures: Vec<u32>, resolution: UVec2) -> Vec<Texture> {
    textures
        .into_iter()
        .map(|texture| create_texture_from_gles(device, texture, resolution))
        .collect()
}
