#!/usr/bin/env python3
import struct,sys,zlib
from pathlib import Path
PNG=b'\x89PNG\r\n\x1a\n'
def decode(path):
 d=Path(path).read_bytes(); assert d.startswith(PNG); pos=8; ids=[]; w=h=depth=color=interlace=None
 while pos<len(d):
  n=struct.unpack('>I',d[pos:pos+4])[0]; k=d[pos+4:pos+8]; p=d[pos+8:pos+8+n]; pos+=12+n
  if k==b'IHDR': w,h,depth,color,_,_,interlace=struct.unpack('>IIBBBBB',p)
  elif k==b'IDAT': ids.append(p)
  elif k==b'IEND': break
 if depth!=8 or color not in (2,6) or interlace!=0: raise ValueError(f'unsupported PNG {depth}/{color}/{interlace}')
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
def chunk(kind,payload):
 return struct.pack('>I',len(payload))+kind+payload+struct.pack('>I',zlib.crc32(kind+payload)&0xffffffff)
def encode(path,rows):
 h=len(rows); w=len(rows[0]); raw=b''.join(b'\x00'+b''.join(bytes(p) for p in row) for row in rows)
 data=PNG+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raw,9))+chunk(b'IEND',b'')
 Path(path).write_bytes(data)
def normalize(src,dst,tw=1536,th=1024):
 w,h,img=decode(src)
 # Trim symmetric compositor shadow to the target aspect before scaling.
 target_ratio=tw/th; ratio=w/h
 if ratio>target_ratio:
  cropw=round(h*target_ratio); x0=(w-cropw)//2; img=[row[x0:x0+cropw] for row in img]; w=cropw
 elif ratio<target_ratio:
  croph=round(w/target_ratio); y0=(h-croph)//2; img=img[y0:y0+croph]; h=croph
 out=[]
 for y in range(th):
  sy=min(h-1,int((y+.5)*h/th)); row=[]
  for x in range(tw):
   sx=min(w-1,int((x+.5)*w/tw)); row.append(img[sy][sx])
  out.append(row)
 encode(dst,out)
if __name__=='__main__':
 if len(sys.argv)!=3: raise SystemExit('usage: v221-normalize-capture.py IN.png OUT.png')
 normalize(sys.argv[1],sys.argv[2])
 print('OPENDECK_V221_CAPTURE_NORMALIZE=PASS')
