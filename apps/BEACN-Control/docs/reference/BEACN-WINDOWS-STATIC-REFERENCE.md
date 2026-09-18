# BEACN Windows static reference set

This project uses the user-supplied Windows installers only as clean-room behavioral and visual references. They are never executed, redistributed, linked into the Linux application, or copied as source.

## Reference installers

| Installer | SHA-256 | Size | Static package observations |
|---|---|---:|---|
| `BEACN+Setup+V1.2.62.0.exe` | `fa21de4f6721fb8bf8d949ca2c48e605a6cfddd36052b2ee8989819b425d78d1` | 122,858,432 bytes | PE32 Advanced Installer bootstrap; 3 valid embedded PNG resources; 2 embedded CAB prerequisite archives |
| `BEACN+Setup+V1.4.1.21.exe` | `08e1c769da7b39115c2e4d09b3daa6bfd9f6ea90b05a6bcc6a437bb504fddecf` | 124,148,536 bytes | PE32 Advanced Installer bootstrap; 4 valid embedded PNG resources; 2 embedded CAB prerequisite archives |
| `BEACN+Link+App+Setup+V1.0.4.0(1).exe` | `1a7cad5d18ae5cd15a45a501359a5fd8753ada30db6e67584600c3381c3598a7` | 59,855,056 bytes | separate PE32 Advanced Installer companion package; 1 common embedded PNG resource; 2 embedded CAB prerequisite archives |

The bootstrap executables contain `Software\\Caphyon\\Advanced Installer\\`, which identifies the packaging engine. The application payload is not exposed as a plain ZIP or standalone source tree by the bootstrap, so this reforge does not claim access to proprietary implementation source.

## Resource delta supported by the files

The 1.2.62 and 1.4.1.21 installers share identical hashes for the 256x256 icon, 400x500 BEACN branding image, and one 148x192 mixer/control image. The 1.4.1.21 installer contains one additional distinct 148x192 mixer/control-style PNG. The Link installer carries the same 256x256 icon resource but is otherwise a separate package.

These images are reference evidence only. AetherForge BEACN Control uses original DragonGlass UI assets and does not ship these embedded Windows resources.

## Clean-room implementation boundary

The Windows files are used to establish product identity, package generations, visual density, mixer/control metaphors, and the existence of a separate Link companion package. Feature behavior already established by the project's documented clean-room parity work remains implemented from original Rust code and independently documented public/device behavior. No proprietary Windows binaries, DLLs, resources, strings, or artwork are copied into the runtime package.

## v0.1.15 safety interpretation

The Linux reforge intentionally does **not** reproduce Windows driver ownership. Linux `snd_usb_audio`, ALSA, and PipeWire keep the physical BEACN device. Direct vendor USB claiming remains blocked. The audible DSP implementation is a new, private Rust signal path that publishes a separate virtual microphone and does not use AetherStream or any AetherForge system-DSP service.
