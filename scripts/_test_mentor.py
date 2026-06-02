#!/usr/bin/env python3
import subprocess, sys, time
p = subprocess.Popen(
    [sys.executable, 'scripts/nursery_mentor.py', '--corpus'],
    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, cwd='/Users/joe/unnamed'
)
time.sleep(10)
p.terminate()
out, _ = p.communicate(timeout=5)
print(out.decode()[:5000] if out else "(no output)")
