#!/usr/bin/env python3
"""Broad-structure PNG comparison for the OpenDeck 2.0.22 human-reviewed visual gate.

This intentionally downsamples the screenshots before comparison so text glyphs, antialiasing,
and small icon differences do not dominate the structural score. It supports 8-bit, non-interlaced
RGB/RGBA PNGs (the canonical render and KDE Spectacle output formats used by qualification).
"""
import math, struct, sys, zlib
from pathlib import Path

PNG=b'\x89PNG\r\n\x1a\n'

def read_png(path):
    data=Path(path).read_bytes()
    if not data.startswith(PNG): raise ValueError(f'{path}: not PNG')
    pos=8; width=height=depth=color=interlace=None; chunks=[]
    while pos < len(data):
        length=struct.unpack('>I',data[pos:pos+4])[0]; kind=data[pos+4:pos+8]; payload=data[pos+8:pos+8+length]; pos += 12+length
        if kind==b'IHDR': width,height,depth,color,_,_,interlace=struct.unpack('>IIBBBBB',payload)
        elif kind==b'IDAT': chunks.append(payload)
        elif kind==b'IEND': break
    if (width,height)!=(1536,1024): raise ValueError(f'{path}: expected 1536x1024, got {width}x{height}')
    if depth!=8 or color not in (2,6) or interlace!=0: raise ValueError(f'{path}: unsupported PNG depth/color/interlace={depth}/{color}/{interlace}')
    bpp=3 if color==2 else 4; stride=width*bpp; raw=zlib.decompress(b''.join(chunks)); rows=[]; prev=bytearray(stride); p=0
    def paeth(a,b,c):
        q=a+b-c; pa=abs(q-a); pb=abs(q-b); pc=abs(q-c)
        return a if pa<=pb and pa<=pc else b if pb<=pc else c
    for _ in range(height):
        ft=raw[p]; p+=1; scan=bytearray(raw[p:p+stride]); p+=stride
        for x in range(stride):
            a=scan[x-bpp] if x>=bpp else 0; b=prev[x]; c=prev[x-bpp] if x>=bpp else 0
            if ft==1: scan[x]=(scan[x]+a)&255
            elif ft==2: scan[x]=(scan[x]+b)&255
            elif ft==3: scan[x]=(scan[x]+((a+b)//2))&255
            elif ft==4: scan[x]=(scan[x]+paeth(a,b,c))&255
            elif ft!=0: raise ValueError(f'{path}: invalid PNG filter {ft}')
        row=[]
        for x in range(width):
            i=x*bpp; row.append((scan[i],scan[i+1],scan[i+2]))
        rows.append(row); prev=scan
    return rows

def block_average(img, cols=64, rows=42):
    h=len(img); w=len(img[0]); out=[]
    for by in range(rows):
        y0=by*h//rows; y1=(by+1)*h//rows
        line=[]
        for bx in range(cols):
            x0=bx*w//cols; x1=(bx+1)*w//cols; sr=sg=sb=n=0
            for y in range(y0,y1):
                for r,g,b in img[y][x0:x1]: sr+=r; sg+=g; sb+=b; n+=1
            line.append((sr/n,sg/n,sb/n))
        out.append(line)
    return out

def luma(c): return .2126*c[0]+.7152*c[1]+.0722*c[2]

def color_score(a,b):
    total=0; n=0
    # Downweight the top-right window-control/text zone and key/icon interiors by using broad block averages.
    for y in range(len(a)):
        for x in range(len(a[0])):
            ca,cb=a[y][x],b[y][x]
            total += (abs(ca[0]-cb[0])+abs(ca[1]-cb[1])+abs(ca[2]-cb[2]))/(3*255)
            n += 1
    return max(0.0,1-total/n)

def gradients(grid):
    h=len(grid); w=len(grid[0]); vals=[[luma(c) for c in row] for row in grid]; out=[]
    for y in range(1,h-1):
        row=[]
        for x in range(1,w-1):
            gx=vals[y][x+1]-vals[y][x-1]; gy=vals[y+1][x]-vals[y-1][x]
            row.append(math.sqrt(gx*gx+gy*gy))
        out.append(row)
    return out

def edge_score(a,b):
    ga,gb=gradients(a),gradients(b); diff=den=0.0
    for y in range(len(ga)):
        for x in range(len(ga[0])):
            # Normalize gradient mismatch by local structural energy + a dark-theme floor.
            diff += abs(ga[y][x]-gb[y][x])
            den += max(ga[y][x],gb[y][x],24.0)
    return max(0.0,1-diff/den)

def main():
    if len(sys.argv)!=3:
        print('usage: v222-png-metrics.py CANONICAL.png ACTUAL.png',file=sys.stderr); return 2
    a=block_average(read_png(sys.argv[1])); b=block_average(read_png(sys.argv[2])); cs=color_score(a,b); es=edge_score(a,b)
    print(f'OPENDECK_V222_COLOR_SIMILARITY={cs:.6f}')
    print(f'OPENDECK_V222_EDGE_SIMILARITY={es:.6f}')
    passed=cs>=.82 and es>=.93
    print('OPENDECK_V222_SCREENSHOT_SIMILARITY='+('PASS' if passed else 'FAIL'))
    return 0 if passed else 1
if __name__=='__main__': raise SystemExit(main())
