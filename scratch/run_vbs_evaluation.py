import subprocess, time, json

testcases = [
    ("graph2.col", "Track 1: SnarkKeyBridge (N=70)"),
    ("graph339.col", "Track 1: SnarkKeyBridge (N=2004, baseline 153k incs timeout)"),
    ("graph10.col", "Track 1 / General (N=142)"),
    ("graph25.col", "Track 1 / General (N=350)"),
    ("graph50.col", "Track 1 / General (N=700)"),
    ("graph100.col", "Track 1 / General (N=1400)"),
    ("graph677.col", "Track 2: GadgetInterfaceParity (N=3868, Class B2b)"),
    ("graph678.col", "Track 2: GadgetInterfaceParity (N=3868, Class B2b)"),
    ("graph744.col", "Track 2: GadgetInterfaceParity (N=4278, Class B2b)"),
    ("graph761.col", "Track 2: GadgetInterfaceParity (N=4430, Class B2b)"),
]

results = []

for filename, desc in testcases:
    cmd = f"taskset -c 0,1,2 nice -n 19 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/{filename} --auto 1 --timeout 45"
    t0 = time.time()
    p = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    elapsed = time.time() - t0
    
    is_sat = "s SATISFIABLE" in p.stdout
    track = "Unknown"
    for line in p.stdout.splitlines():
        if "Track:" in line:
            track = line.split("Track:")[1].strip()
            break
            
    results.append({
        "file": filename,
        "description": desc,
        "track": track,
        "is_sat": is_sat,
        "elapsed_sec": round(elapsed, 2),
        "returncode": p.returncode,
    })
    status_str = "SOLVED (SAT)" if is_sat else "TIMEOUT / UNSAT"
    print(f"{filename:15} | {status_str:15} | Time: {elapsed:6.2f}s | Track: {track}")

with open("scratch/vbs_eval_summary.json", "w") as f:
    json.dump(results, f, indent=2)
