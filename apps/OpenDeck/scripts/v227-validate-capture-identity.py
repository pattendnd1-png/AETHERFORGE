#!/usr/bin/env python3
import json, struct, sys, zlib
from pathlib import Path

PNG=b'\x89PNG\r\n\x1a\n'
TARGET=(17,241,109)
TOL=12

def decode(path: Path):
    data=path.read_bytes()
    if not data.startswith(PNG): raise ValueError('not PNG')
    pos=8; ids=[]; w=h=depth=color=interlace=None
    while pos < len(data):
        n=struct.unpack('>I',data[pos:pos+4])[0]; k=data[pos+4:pos+8]; p=data[pos+8:pos+8+n]; pos += 12+n
        if k==b'IHDR': w,h,depth,color,_,_,interlace=struct.unpack('>IIBBBBB',p)
        elif k==b'IDAT': ids.append(p)
        elif k==b'IEND': break
    if depth != 8 or color not in (2,6) or interlace != 0: raise ValueError(f'unsupported PNG {depth}/{color}/{interlace}')
    bpp=3 if color==2 else 4; stride=w*bpp; raw=zlib.decompress(b''.join(ids)); rows=[]; prev=bytearray(stride); q=0
    def paeth(a,b,c):
        r=a+b-c; pa=abs(r-a); pb=abs(r-b); pc=abs(r-c); return a if pa<=pb and pa<=pc else b if pb<=pc else c
    for _ in range(h):
        ft=raw[q]; q+=1; scan=bytearray(raw[q:q+stride]); q+=stride
        for x in range(stride):
            a=scan[x-bpp] if x>=bpp else 0; b=prev[x]; c=prev[x-bpp] if x>=bpp else 0
            if ft==1: scan[x]=(scan[x]+a)&255
            elif ft==2: scan[x]=(scan[x]+b)&255
            elif ft==3: scan[x]=(scan[x]+((a+b)//2))&255
            elif ft==4: scan[x]=(scan[x]+paeth(a,b,c))&255
            elif ft!=0: raise ValueError(f'bad filter {ft}')
        rows.append([tuple(scan[i:i+3]) for i in range(0,stride,bpp)]); prev=scan
    return w,h,rows

def near(c): return all(abs(c[i]-TARGET[i]) <= TOL for i in range(3))
def count_region(img,x0,y0,x1,y1): return sum(1 for y in range(y0,y1) for c in img[y][x0:x1] if near(c))

def main():
    if len(sys.argv)!=5: raise SystemExit('usage: v227-validate-capture-identity.py IMAGE.png FOCUS_ACK.json EXPECTED_PID EXPECTED_RELEASE')
    image=Path(sys.argv[1]); ack_path=Path(sys.argv[2]); pid=int(sys.argv[3]); release=sys.argv[4]
    ack=json.loads(ack_path.read_text())
    if ack.get('release') != release or int(ack.get('pid',-1)) != pid:
        raise SystemExit(f'OPENDECK_V227_CAPTURE_IDENTITY=FAIL:ACK:{ack}')
    w,h,img=decode(image)
    if (w,h)!=(1536,1024): raise SystemExit(f'OPENDECK_V227_CAPTURE_IDENTITY=FAIL:SIZE:{w}x{h}')
    nw=count_region(img,0,0,96,96)
    se=count_region(img,w-96,h-96,w,h)
    if nw < 8 or se < 8:
        raise SystemExit(f'OPENDECK_V227_CAPTURE_IDENTITY=FAIL:TOKEN:nw={nw}:se={se}')
    print(f'OPENDECK_V227_CAPTURE_IDENTITY=PASS:release={release}:pid={pid}:nw={nw}:se={se}')
if __name__=='__main__': main()
