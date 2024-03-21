# alvr-web research

Lets try to port ALVR to Web browser!

NOTE I'm writing this project as proof of concept mode (prefer to reach to "just works" state faster as possible (to reduce chance of get bored me and abandoned) rather than code quality), my code quality is terrible

* alvr/web_gateway: WebSocket server, communicates with WebXR browser and ALVR server (SteamVR)
  * Run `cargo run --bin web_gateway` to start server
* web_client: WebXR client, communicates with web_gateway
  * Prerequisite: Node.js, pnpm (If you don't have `pnpm` command, I'll recommend [corepack](https://nodejs.org/api/corepack.html), it is like Rustup in Cargo but you need to install Node.js since corepack is only for package maangers)
  * `pnpm install` to install dependencies
  * going to `web_client` and Run `pnpm vite` to start dev server

Warning, WebXR (and/or) WebCodecs requires "secure context", so you can't get work with HTTP, like `http://192.168.1.5`.
* If you are using default browser of Oculus/Meta HMD (which is Chromium-base), you can use Chrome port forwarding from open `chrome://inspect/#devices` in your PC
  * You need to forwarding two port, `5173` (for Vite dev server) and `5999` (for web_gateway)
* Otherwise, you need to find a way to get HTTPS, or port forwarding to target device's localhost (almost every browser considers `http://localhost` as secure context)

---

<p align="center"> <img width="500" src="resources/alvr_combined_logo_hq.png"/> </p>

# ALVR - Air Light VR

[![badge-discord][]][link-discord] [![badge-matrix][]][link-matrix] [![badge-opencollective][]][link-opencollective]

Stream VR games from your PC to your headset via Wi-Fi.  
ALVR uses technologies like [Asynchronous Timewarp](https://developer.oculus.com/documentation/native/android/mobile-timewarp-overview) and [Fixed Foveated Rendering](https://developer.oculus.com/documentation/native/android/mobile-ffr) for a smoother experience.  
Most of the games that run on SteamVR or Oculus Software (using Revive) should work with ALVR.  
This is a fork of [ALVR](https://github.com/polygraphene/ALVR).

|      VR Headset       |                                Support                                 |
| :-------------------: | :--------------------------------------------------------------------: |
|    Quest 1/2/3/Pro    |                           :heavy_check_mark:                           |
|     Pico 4/Neo 3      |                           :heavy_check_mark:                           |
| Vive Focus 3/XR Elite |                           :heavy_check_mark:                           |
|        YVR 1/2        |                           :heavy_check_mark:                           |
|        Lynx R1        |                           :heavy_check_mark:                           |
|   Smartphone/Monado   |                              :warning: *                               |
|   Google Cardboard    | :warning: * ([PhoneVR](https://github.com/PhoneVR-Developers/PhoneVR)) |
|        GearVR         |                         :construction: (maybe)                         |
|       Oculus Go       |                                 :x: **                                 |

\* : Only works on some smartphones, not enough testing.  
\** : Oculus Go support was dropped, the minimum supported OS is Android 8. Download the last compatible version [here](https://github.com/alvr-org/ALVR/releases/tag/v18.2.3).

|        PC OS        |       Support       |
| :-----------------: | :-----------------: |
|   Windows 8/10/11   | :heavy_check_mark:  |
|    Windows 7/XP     |         :x:         |
|     Ubuntu/Arch     |    :warning: ***    |
| Other linux distros | :grey_question: *** |
|        macOS        |         :x:         |

\*** : Linux support is still in beta. To be able to make audio work or run ALVR at all you may need advanced knowledge of your distro for debugging or building from source.

## Requirements

-   A supported standalone VR headset (see compatibility table above)

-   SteamVR

-   High-end gaming PC
    -   See OS compatibility table above.
    -   NVIDIA GPU that supports NVENC (1000 GTX Series or higher) (or with an AMD GPU that supports AMF VCE) with the latest driver.
    -   Laptops with an onboard (Intel HD, AMD iGPU) and an additional dedicated GPU (NVidia GTX/RTX, AMD HD/R5/R7): you should assign the dedicated GPU or "high performance graphics adapter" to the applications ALVR, SteamVR for best performance and compatibility. (NVidia: Nvidia control panel->3d settings->application settings; AMD: similiar way)

-   802.11ac 5Ghz wireless or ethernet wired connection  
    -   It is recommended to use 802.11ac 5Ghz for the headset and ethernet for PC  
    -   You need to connect both the PC and the headset to same router (or use a routed connection as described [here](https://github.com/alvr-org/ALVR/wiki/ALVR-v14-and-Above))

## Install

Follow the installation guide [here](https://github.com/alvr-org/ALVR/wiki/Installation-guide).

## Troubleshooting

-   Please check the [Troubleshooting](https://github.com/alvr-org/ALVR/wiki/Troubleshooting) page. The original repository [wiki](https://github.com/polygraphene/ALVR/wiki/Troubleshooting) can also help.  
-   Configuration recommendations and information may be found [here](https://github.com/alvr-org/ALVR/wiki/PC)

## Uninstall

Open `ALVR Dashboard.exe`, go to `Installation` tab then press `Remove firewall rules`. Close ALVR window and delete the ALVR folder.

## Build from source

You can follow the guide [here](https://github.com/alvr-org/ALVR/wiki/Building-From-Source).

## License

ALVR is licensed under the [MIT License](LICENSE).

## Privacy policy

ALVR apps do not directly collect any kind of data.

## Donate

If you want to support this project you can make a donation to our [Open Source Collective account](https://opencollective.com/alvr).

You can also donate to the original author of ALVR using Paypal (polygraphene@gmail.com) or with bitcoin (1FCbmFVSjsmpnAj6oLx2EhnzQzzhyxTLEv).

[badge-discord]: https://img.shields.io/discord/720612397580025886?style=for-the-badge&logo=discord&color=5865F2 "Join us on Discord"
[link-discord]: https://discord.gg/ALVR
[badge-matrix]: https://img.shields.io/static/v1?label=chat&message=%23alvr&style=for-the-badge&logo=matrix&color=blueviolet "Join us on Matrix"
[link-matrix]: https://matrix.to/#/#alvr:ckie.dev?via=ckie.dev
[badge-opencollective]: https://img.shields.io/opencollective/all/alvr?style=for-the-badge&logo=opencollective&color=79a3e6 "Donate"
[link-opencollective]: https://opencollective.com/alvr
