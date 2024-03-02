import { BitReader } from "./bit_reader"
import { parseInitNALUHEVC } from "./parser/hevc"
import { srcFragmentShader, srcVertexShader } from "./shader"

class DiveSession {
    ws: WebSocket
    videoDecoder: VideoDecoder | undefined
    spsAndPps: Uint8Array | undefined
    lastFrame: VideoFrame | undefined

    glLayer = new XRWebGLLayer(this.session, this.gl)
    referenceSpace!: XRReferenceSpace | XRBoundedReferenceSpace

    constructor(public session: XRSession, public gl: WebGLRenderingContext) {
        console.log("session started", session)
        session.updateRenderState({
            baseLayer: this.glLayer,
        })
        this.ws = new WebSocket(`ws://${location.hostname}:5999/websocket`)
        this.openWebSocket()
    }

    async setup() {
        try {
            let rate = this.session.frameRate ?? 0
            for (const r of this.session.supportedFrameRates ?? []) {
                if (r > rate) rate = r
            }
            console.log("update rate to ", rate)
            // await this.session.updateTargetFrameRate(rate)
        } catch(e) {
            console.error(e)
        }
        this.referenceSpace = await this.session.requestReferenceSpace("local-floor")
        const vs = this.gl.createShader(this.gl.VERTEX_SHADER)
        if (vs == null) throw new Error("vs fail")
        this.gl.shaderSource(vs, srcVertexShader)
        this.gl.compileShader(vs)
        if (!this.gl.getShaderParameter(vs, this.gl.COMPILE_STATUS)) {
            throw new Error(this.gl.getShaderInfoLog(vs) ?? "")
        }
        const fs = this.gl.createShader(this.gl.FRAGMENT_SHADER)
        if (fs == null) throw new Error("fs fail")
        this.gl.shaderSource(fs, srcFragmentShader)
        this.gl.compileShader(fs)
        if (!this.gl.getShaderParameter(fs, this.gl.COMPILE_STATUS)) {
            throw new Error(this.gl.getShaderInfoLog(fs) ?? "")
        }

        const program = this.gl.createProgram()
        if (program == null) throw new Error("program fail")
        this.gl.attachShader(program, vs)
        this.gl.attachShader(program, fs)
        this.gl.linkProgram(program)
        if (!this.gl.getProgramParameter(program, this.gl.LINK_STATUS)) {
            throw new Error(this.gl.getProgramInfoLog(program) ?? "")
        }
        this.gl.useProgram(program)

        const texCoord = this.gl.getAttribLocation(program, "a_texCoord")
        this.gl.enableVertexAttribArray(texCoord)
        const uvPos = new Float32Array([0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0])
        this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.gl.createBuffer())
        this.gl.bufferData(this.gl.ARRAY_BUFFER, uvPos, this.gl.STATIC_DRAW)
        this.gl.vertexAttribPointer(texCoord, 2, this.gl.FLOAT, false, 0, 0)
        this.gl.activeTexture(this.gl.TEXTURE0)
        this.gl.bindTexture(this.gl.TEXTURE_2D, this.gl.createTexture())
        this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_MIN_FILTER, this.gl.LINEAR)
        this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_S, this.gl.CLAMP_TO_EDGE)
        this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_T, this.gl.CLAMP_TO_EDGE)
        this.gl.uniform1i(this.gl.getUniformLocation(program, "sampler0"), 0)
        this.gl.uniform1f(this.gl.getUniformLocation(this.gl.getParameter(this.gl.CURRENT_PROGRAM), "width"), 0.5)
        this.session.requestAnimationFrame(this.raf)
    }

    texture = new Uint8Array(0)

    raf: XRFrameRequestCallback = (time, frame) => {

        // console.log(time, frame)
        if (this.ws.readyState === this.ws.OPEN) {
            this.ws.send(`alive:${Math.floor(time * 1000)}`)
        }

        this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, this.glLayer.framebuffer)
        const pose = frame.getViewerPose(this.referenceSpace)
        if (pose == null) return console.log("pose is null")

        if (this.ws.readyState === this.ws.OPEN) {
            const buffer = new ArrayBuffer(
                4 * Uint32Array.BYTES_PER_ELEMENT +
                1 * BigUint64Array.BYTES_PER_ELEMENT +
                3 * Float32Array.BYTES_PER_ELEMENT +
                4 * Float32Array.BYTES_PER_ELEMENT +
                3 * Float32Array.BYTES_PER_ELEMENT +
                3 * Float32Array.BYTES_PER_ELEMENT +
                (4 * 2) * Float32Array.BYTES_PER_ELEMENT +
                0
            )
            const view = new DataView(buffer)
            view.setUint32(0, 1, true)
            view.setBigUint64(4, BigInt(Math.floor(time * 1000)), true)
            const msg = new Float32Array(buffer, 12)
            msg[0] = pose.transform.orientation.x
            msg[1] = pose.transform.orientation.y
            msg[2] = pose.transform.orientation.z
            msg[3] = pose.transform.orientation.w

            msg[4] = pose.transform.position.x
            msg[5] = pose.transform.position.y
            msg[6] = pose.transform.position.z

            const linearVelocity = pose.linearVelocity
            if (linearVelocity != null) {
                msg[7] = linearVelocity.x
                msg[8] = linearVelocity.y
                msg[9] = linearVelocity.z
            }

            const angularVelocity = pose.angularVelocity
            if (angularVelocity != null) {
                msg[10] = angularVelocity.x
                msg[11] = angularVelocity.y
                msg[12] = angularVelocity.z
            }

            // I'm not sure those are correct
            const leftEye = pose.views.find(v => v.eye === "left")
            if (leftEye != null) {
                const fovX = Math.atan(1 / leftEye.projectionMatrix[0]) * 2
                const fovY = Math.atan(1 / leftEye.projectionMatrix[5]) * 2
                msg[13] = -fovX
                msg[14] = fovX
                msg[15] = -fovY
                msg[16] = fovY
            }
            const rightEye = pose.views.find(v => v.eye === "right")
            if (rightEye != null) {
                const fovX = Math.atan(1 / rightEye.projectionMatrix[0]) * 2
                const fovY = Math.atan(1 / rightEye.projectionMatrix[5]) * 2
                msg[17] = -fovX
                msg[18] = fovX
                msg[19] = -fovY
                msg[20] = fovY
            }

            this.ws.send(buffer)
        }

        for (const view of pose.views) {
            const viewport = this.glLayer.getViewport(view)
            if (viewport == null) continue
            this.gl.viewport(viewport.x, viewport.y, viewport.width, viewport.height)
            if (this.lastFrame != null) {
                this.gl.activeTexture(this.gl.TEXTURE0)
                this.gl.texImage2D(this.gl.TEXTURE_2D, 0, this.gl.RGBA, this.lastFrame.codedWidth, this.lastFrame.codedHeight, 0, this.gl.RGBA, this.gl.UNSIGNED_BYTE, null)
                this.gl.texSubImage2D(this.gl.TEXTURE_2D, 0, 0, 0, this.gl.RGBA, this.gl.UNSIGNED_BYTE, this.lastFrame)
            }
            const isLeft = view.eye === "left"
            this.gl.uniform1f(this.gl.getUniformLocation(this.gl.getParameter(this.gl.CURRENT_PROGRAM), "left"), isLeft ? 0 : 0.5)
            this.gl.drawArrays(this.gl.TRIANGLE_FAN, 0, 4)
            this.gl.flush()
        }

        this.session.requestAnimationFrame(this.raf)
    }
    
    openWebSocket() {
        this.ws.binaryType = "arraybuffer"
        
        this.ws.addEventListener("open", e => {
            this.ws.send("hello")
            this.ws.send(`alive:${Math.floor(performance.now() * 1000)}`)
        })
        
        const canvas = document.createElement("canvas")
        document.body.appendChild(canvas)
        
        this.ws.addEventListener("message", this.onWebSocketMessage)
        this.ws.addEventListener("close", () => {
            this.session.end()
        })
        this.ws.addEventListener("error", () => {
            this.session.end()
        })
        this.session.addEventListener("end", () => {
            this.ws.close()
        })
    }

    onWebSocketMessage = (e: MessageEvent) => {
        if (typeof e.data === "string") {
            console.log(e.data)
        } else if (e.data instanceof ArrayBuffer) {
            const arr = new DataView(e.data)
            const messageType = arr.getUint32(0, true);
            console.log(messageType)
            if (messageType === 1) {
                // frame ready
                // u128 timestamp
                // ... nal
                if (this.videoDecoder == null) {
                    // we can't do anything if decoder is missing
                    return
                }
                const timestamp = arr.getBigUint64(8, true) | (arr.getBigUint64(16, true) << 32n)
                let nal: Uint8Array
                if (this.spsAndPps != null) {
                    nal = new Uint8Array(e.data.byteLength - 24 + this.spsAndPps.byteLength)
                    nal.set(this.spsAndPps, 0)
                    nal.set(new Uint8Array(e.data, 24), this.spsAndPps.byteLength)
                } else {
                    nal = new Uint8Array(e.data, 24)
                }
                const nalReader = new BitReader(new Uint8Array(e.data, 24))
                let count = 0
                while (nalReader.read(8) === 0) count++
                if (count < 2) {
                    alert("invalid nal: not starting with 0x000001")
                    this.ws.close()
                    return
                }
                if (nalReader.read(1) !== 0) {
                    alert("invalid nal: forbidden_zero_bit not 0")
                    return
                }
                const isKey = nalReader.read(2)
                console.log(this.spsAndPps, isKey, timestamp)
                if (this.videoDecoder != null) {
                    const init = {
                        type: this.spsAndPps != null ? "key" : "delta",
                        timestamp: Number(timestamp),
                        data: nal,
                        // transfer: [nal.buffer],
                    } satisfies EncodedVideoChunkInit
                    this.videoDecoder.decode(new EncodedVideoChunk(init))
                    this.spsAndPps = undefined
                } else {
                    this.ws.send("idr")
                }
            }
            if (messageType === 2) {
                // create decoder
                // u32 codec
                // ... nal
                const codec = arr.getUint32(4, true)
                if (this.videoDecoder != null) {
                    if (this.videoDecoder.state !== "closed") this.videoDecoder.close()
                }
                const initNal = new Uint8Array(e.data, 8)
                console.log(initNal)
                if (codec === 0) {
                    this.initializeH264(initNal)
                } else if (codec === 1) {
                    this.initializeHEVC(initNal)
                } else {
                    alert(`unsupported codec: ${codec}`)
                    return
                }
            }
        }
    }

    initializeH264(initNal: Uint8Array) {
        // read SPS
        const initNalReader = new BitReader(initNal)
        let count = 0
        while (initNalReader.read(8) === 0) count++
        if (count < 2) {
            alert("invalid initNal: not starting with 0x000001")
            this.ws.close()
            return
        }
        if (initNalReader.read(1) !== 0) {
            alert("invalid initNal: forbidden_zero_bit not 0")
            return
        }
        if (initNalReader.read(2) !== 0b11) {
            alert("invalid initNal: nal_ref_idc not 0b11")
            return
        }
        if (initNalReader.read(5) !== 7) {
            alert("invalid initNal: nal_unit_type not 7 (SPS)")
            return
        }
        const profile_idc = initNalReader.read(8)
        const constraint_flags = initNalReader.read(8)
        const level_idc = initNalReader.read(8)
        const codecString = `avc1.${[profile_idc, constraint_flags, level_idc].map(x => x.toString(16).padStart(2, "0")).join("")}`
        const seq_parameter_set_id = initNalReader.uev()
        console.log(seq_parameter_set_id)

        console.log(codecString)


        this.videoDecoder = new VideoDecoder({
            output: (frame) => {
                this.ws.send(`decoded:${frame.timestamp}`)
                this.lastFrame?.close()
                this.lastFrame = frame
            },
            error: (e) => {
                console.error(e)
                this.ws.send("idr")
                this.videoDecoder = undefined
            }
        })
        this.videoDecoder.configure({
            codec: codecString,
            optimizeForLatency: true,
        })
        this.spsAndPps = initNal
    }

    initializeHEVC(initNal: Uint8Array) {
        try {
            const codecString = parseInitNALUHEVC(initNal)

            this.videoDecoder = new VideoDecoder({
                output: (frame) => {
                    this.ws.send(`decoded:${frame.timestamp}`)
                    this.lastFrame?.close()
                    this.lastFrame = frame
                },
                error: (e) => {
                    console.error(e)
                    this.ws.send("idr")
                    this.videoDecoder = undefined
                }
            })
            console.log(codecString)
            this.videoDecoder.configure({
                codec: codecString,
                optimizeForLatency: true,
            })
            this.spsAndPps = initNal
        } catch (e) {
            console.error(e)
            alert(e)
            this.ws.close()
            return
        }
    }
}

const button = document.createElement("button")
button.textContent = "LINK START"
button.style.fontSize = "15vw"
if (navigator.xr == null) {
    button.textContent = "NO WEBXR"
    button.disabled = true
} else if (!await navigator.xr.isSessionSupported("immersive-vr")) {
    button.textContent = "NOT SUPPORTING immersive-vr"
    button.disabled = true
} else {
    button.addEventListener("click", () => {
        const canvas = document.createElement("canvas")
        const gl = canvas.getContext("webgl", { xrCompatible: true })
        if (gl == null) {
            alert("failed to create webgl context")
            return
        }
        document.body.appendChild(canvas)
        navigator.xr?.requestSession("immersive-vr", {
            // domOverlay: { root: document.body }
            requiredFeatures: ["local-floor"],
        }).then(async session => {
            const dive = new DiveSession(session, gl)
            await dive.setup()
        }).catch(e => {
            console.error(e)
            alert(`failed to start XR session: ${e}`)
        })
    })
}
document.body.appendChild(button)