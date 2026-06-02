#!/usr/bin/env python3
"""解码全感官跑的生命信号为一行 CSV。
用法: decode_full30.py <runtime_dir> <t_min>
列: t_min,reserve,supply,drain,anxiety,pain,vital_state,self_coh,agency,n_obj,top_fam,top_recog,phot_health,hearing_fault,vocal_breath
"""
import struct, os, sys

rt = sys.argv[1]
tmin = sys.argv[2] if len(sys.argv) > 2 else "?"

def f(name): return os.path.join(rt, name)

# --- vital_core.bin: magic,ver,reserve,anxiety,pain,state(u8) ---
reserve = anxiety = pain = -1.0; vstate = -1
supply = drain = -1.0
try:
    b = open(f("vital_core.bin"), "rb").read()
    reserve, anxiety, pain = struct.unpack_from("<ddd", b, 16)
    vstate = b[16 + 24]
    if len(b) >= 41 + 16:  # v2: last_supply/last_drain appended after state byte
        supply, drain = struct.unpack_from("<dd", b, 41)
except Exception: pass

# --- self_model.bin: magic,ver,seed,stab,coherence,agency ---
coh = ag = -1.0
try:
    b = open(f("self_model.bin"), "rb").read()
    _seed, _stab, coh, ag = struct.unpack_from("<dddd", b, 16)
except Exception: pass

# --- familiar_objects.bin: magic,ver,n(u32),[fp×4,fam,rec(u32),last(u64)]... ---
n = top_fam = -1.0; top_rec = -1
try:
    b = open(f("familiar_objects.bin"), "rb").read()
    n = struct.unpack_from("<I", b, 16)[0]
    off = 20; tf = 0.0; tr = 0
    for _ in range(n):
        off += 32  # fingerprint 4×f64
        fam = struct.unpack_from("<d", b, off)[0]; off += 8
        rec = struct.unpack_from("<I", b, off)[0]; off += 4
        off += 8  # last_seen u64
        if fam > tf: tf = fam; tr = rec
    top_fam, top_rec = tf, tr
except Exception: pass

# --- snapshot.bin: phot_health @ byte offset 152 (u64, take low byte),
#     hearing_fault @ offset 88 (u64) ---
phot_health = hearing_fault = -1
vocal_breath = -1.0
try:
    b = open(f("snapshot.bin"), "rb").read()
    hearing_fault = struct.unpack_from("<Q", b, 88)[0]   # fault flag (0/1)
    phot_health = struct.unpack_from("<Q", b, 152)[0]    # 0=Fault 1=Quiet 2=Ok 3=Stale
except Exception: pass

def fmt(x, p=3):
    try: return f"{x:.{p}f}"
    except Exception: return str(x)

print(",".join([
    str(tmin), fmt(reserve), fmt(supply), fmt(drain), fmt(anxiety), fmt(pain),
    str(vstate), fmt(coh), fmt(ag), str(int(n)) if n>=0 else "-1",
    fmt(top_fam), str(top_rec), str(phot_health), str(hearing_fault),
    fmt(vocal_breath),
]))
