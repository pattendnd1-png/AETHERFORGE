#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import base64
import difflib
import gzip
import hashlib
import io
import os
import subprocess
import sys
import tarfile
import tempfile
from typing import NoReturn

CANONICAL_SHA="df7844511c2bbaa5b9ac94c11f7c2f0f7c167a2f6d0edc7c4e677e8e6919f1bd"
OUTER_MARKER="base64 -d <<'__AETHER_CC_061__' | tar -xzf - -C \"$tmp\"\n"
OUTER_END="\n__AETHER_CC_061__"
INNER_NAME="INSTALL-AETHER-CONTROL-CENTER-v0.6.1.sh"

OLD_GUARD='  [[ "${ID:-}" == "aetherforge" && "${VERSION_ID:-}" == "7.0.1" ]] || {'
NEW_GUARD='  [[ "${ID:-}" == "aetherforge" && "${VERSION_ID:-}" == "7.0.7" ]] || {'
OLD_ERROR="    echo 'ERROR: target must be AetherForge 7.0.1' >&2"
NEW_ERROR="    echo 'ERROR: target must be AetherForge 7.0.7' >&2"

def fail(msg: str) -> NoReturn:
    raise SystemExit("ERROR: "+msg)

def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def safe_extract(raw: bytes, dst: Path) -> None:
    dst=dst.resolve()
    dst.mkdir(parents=True,exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(raw),mode="r:gz") as tf:
        for member in tf.getmembers():
            target=(dst/member.name).resolve()
            if not (target==dst or str(target).startswith(str(dst)+os.sep)):
                fail("unsafe Control Center member: "+member.name)
        tf.extractall(dst,filter="data")

def deterministic_tar(root: Path) -> bytes:
    buf=io.BytesIO()
    def filt(info: tarfile.TarInfo) -> tarfile.TarInfo:
        info.uid=info.gid=0
        info.uname=info.gname="root"
        info.mtime=0
        info.mode &= 0o777
        return info
    with gzip.GzipFile(fileobj=buf,mode="wb",compresslevel=9,mtime=0) as gz:
        with tarfile.open(fileobj=gz,mode="w",format=tarfile.PAX_FORMAT) as tf:
            tf.add(root,arcname=root.name,recursive=True,filter=filt)
    return buf.getvalue()

def split_outer(data: bytes) -> tuple[str,int,int,bytes]:
    try:
        text=data.decode()
    except UnicodeDecodeError as exc:
        fail(f"canonical runtime installer is not UTF-8 shell text: {exc}")
    start=text.find(OUTER_MARKER)
    if start<0:
        fail("Control Center outer payload marker missing")
    payload_start=start+len(OUTER_MARKER)
    end=text.find(OUTER_END,payload_start)
    if end<0:
        fail("Control Center outer payload end marker missing")
    if text.find(OUTER_MARKER,payload_start)>=0:
        fail("duplicate Control Center outer payload marker")
    try:
        raw=base64.b64decode(''.join(text[payload_start:end].split()),validate=True)
    except Exception as exc:
        fail(f"Control Center outer payload is invalid base64: {exc}")
    return text,payload_start,end,raw

def validate_change(before: str, after: str) -> None:
    old_lines=before.splitlines()
    new_lines=after.splitlines()
    if len(old_lines)!=len(new_lines):
        fail("inner installer line count changed unexpectedly")
    changes=[(a,b) for a,b in zip(old_lines,new_lines) if a!=b]
    expected=[(OLD_GUARD,NEW_GUARD),(OLD_ERROR,NEW_ERROR)]
    if changes!=expected:
        diff='\n'.join(difflib.unified_diff(
            old_lines,new_lines,fromfile="canonical-inner",tofile="v7.0.7-inner",lineterm=""
        ))
        fail("unexpected inner installer diff:\n"+diff)

def extract_inner(raw: bytes, dst: Path) -> tuple[Path,Path]:
    safe_extract(raw,dst)
    roots=[p for p in dst.iterdir() if p.is_dir()]
    if len(roots)!=1:
        fail(f"expected one Control Center source root, found {len(roots)}")
    inner=roots[0]/INNER_NAME
    if not inner.is_file():
        fail("Control Center inner installer missing")
    return roots[0],inner

def adapt(inp: Path, out: Path) -> None:
    canonical=inp.read_bytes()
    if digest(canonical)!=CANONICAL_SHA:
        fail("Control Center runtime input is not the canonical v0.6.1 installer")

    outer,payload_start,payload_end,raw=split_outer(canonical)
    with tempfile.TemporaryDirectory(prefix="aetherforge-af36-cc-") as td_raw:
        td=Path(td_raw)
        source_root,inner=extract_inner(raw,td/"canonical")
        before=inner.read_text()
        if before.count(OLD_GUARD)!=1:
            fail(f"expected exactly one v7.0.1 target guard, found {before.count(OLD_GUARD)}")
        if before.count(OLD_ERROR)!=1:
            fail(f"expected exactly one v7.0.1 target error, found {before.count(OLD_ERROR)}")
        if NEW_GUARD in before or NEW_ERROR in before:
            fail("canonical inner installer unexpectedly already contains v7.0.7 target adaptation")

        after=before.replace(OLD_GUARD,NEW_GUARD,1).replace(OLD_ERROR,NEW_ERROR,1)
        validate_change(before,after)
        inner.write_text(after)
        inner.chmod(0o755)

        syntax=subprocess.run(["bash","-n",str(inner)],text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        if syntax.returncode:
            fail("adapted inner installer shell syntax failed: "+syntax.stderr.strip())
        repacked=deterministic_tar(source_root)

    encoded=base64.b64encode(repacked).decode()
    encoded='\n'.join(encoded[i:i+76] for i in range(0,len(encoded),76))
    adapted_text=outer[:payload_start]+encoded+outer[payload_end:]
    out.write_text(adapted_text)
    out.chmod(0o755)

    syntax=subprocess.run(["bash","-n",str(out)],text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    if syntax.returncode:
        fail("adapted outer installer shell syntax failed: "+syntax.stderr.strip())

    staged=out.read_bytes()
    _,_,_,staged_raw=split_outer(staged)
    with tempfile.TemporaryDirectory(prefix="aetherforge-af36-verify-") as td_raw:
        td=Path(td_raw)
        _,staged_inner=extract_inner(staged_raw,td/"staged")
        staged_text=staged_inner.read_text()
        if staged_text.count(NEW_GUARD)!=1 or staged_text.count(NEW_ERROR)!=1:
            fail("staged inner installer lacks exact v7.0.7 target contract")
        if OLD_GUARD in staged_text or OLD_ERROR in staged_text:
            fail("staged inner installer still contains v7.0.1 target contract")

    if digest(inp.read_bytes())!=CANONICAL_SHA:
        fail("canonical runtime installer changed during adaptation")
    print("AF36_CONTROL_CENTER_CANONICAL_RUNTIME_SHA="+CANONICAL_SHA)
    print("AF36_CONTROL_CENTER_STAGED_RUNTIME_SHA="+digest(staged))
    print("AF36_CONTROL_CENTER_V707_ADAPTER=PASS")

def main() -> int:
    if len(sys.argv)!=3:
        fail(f"usage: {Path(sys.argv[0]).name} CANONICAL-INSTALLER OUTPUT-INSTALLER")
    inp=Path(sys.argv[1]).resolve()
    out=Path(sys.argv[2]).resolve()
    if not inp.is_file():
        fail("canonical Control Center runtime installer missing")
    if inp==out:
        fail("refusing to overwrite canonical Control Center runtime installer")
    out.parent.mkdir(parents=True,exist_ok=True)
    adapt(inp,out)
    return 0

if __name__=="__main__":
    raise SystemExit(main())
