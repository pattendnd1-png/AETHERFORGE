from pathlib import Path
all='\n'.join(Path(p).read_text() for p in ['crates/forgehx-gui/src/main.rs','crates/forgehx-gui/src/nav.rs','crates/forgehx-gui/src/app.rs'] if Path(p).exists())
out=Path('crates/forgehx-gui/src/output_dsp.rs')
if not out.exists(): raise SystemExit('FAIL output_dsp page missing')
s=all+out.read_text()
need=['Page::OutputDsp','Output DSP','OutputDspLiveUpdate','OutputDeviceClass::ALL','Bass boost','Clarity boost','Gaming impact','Limiter','PRESERVED_AUDIO_ENDPOINT']
miss=[x for x in need if x not in s]
if miss: raise SystemExit('FAIL missing '+', '.join(miss))
print('FORGEHX_10_0_18_OUTPUT_GUI=PASS')
