# AetherStream System Audio Authority + ForgeHX Bridge Design

AetherStream v10.2.32 is the persistent AetherForge system audio authority. `aetherstream-audiod` owns the application-facing default sink and the single application-facing default microphone source independent of the streaming UI lifecycle.

ForgeHX remains the microphone hardware/DSP authority. It captures the physical HyperX microphone directly, performs its native Rust microphone DSP, and sends post-DSP 10 ms PCM frames to AetherStream through a local nonblocking Unix datagram bridge. ForgeHX no longer publishes or selects an application-facing PipeWire source.

Bridge contract: socket `$XDG_RUNTIME_DIR/aetherstream/forgehx-mic.sock`; 48 kHz; mono; 480 `f32` samples per 10 ms frame; fixed little-endian packet with magic `AFXHXM01`, version 1, sample rate, channels, frame sample count, sequence, followed by 480 samples. Send failure or absent consumer must never block ForgeHX DSP.

AetherStream captures the selected physical microphone as a fallback while receiving ForgeHX packets. The managed source `aetherstream.system.microphone` publishes ForgeHX frames while the bridge heartbeat is live and automatically falls back to direct physical capture if ForgeHX stops. Applications see only the AetherStream system microphone. AetherStream restores the original physical default source on clean shutdown.

Output ownership from v10.2.31 remains unchanged: all application output flows through AetherStream's live Rust output DSP and then to the physical sink.

Safety/real-time rules: no allocation, blocking socket operation, filesystem access, process launch, or mutex wait in the ForgeHX post-DSP frame send path; AetherStream PipeWire callbacks use only bounded ring buffers and atomics. Bridge loss is fail-open to the selected physical microphone.
