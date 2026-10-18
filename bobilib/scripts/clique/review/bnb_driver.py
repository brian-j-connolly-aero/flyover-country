"""Run rbnb on every (instance, size vector, budget) case listed by `mylb.py GRAPHDIR arith`.

usage: bnb_driver.py ARITH_JSONL RBNB_BINARY COMPDIR > bnb_lb.jsonl
"""
import json
import subprocess
import sys
import time

arith, binary, comp = sys.argv[1:4]
for line in open(arith):
    r = json.loads(line)
    for s, b in r.get('size_vectors', []):
        t0 = time.time()
        p = subprocess.run([binary, 'decide', '%s/%s.comp' % (comp, r['name']), str(s[0]), str(s[1]),
                            str(s[2]), str(b)], capture_output=True, text=True, timeout=1200)
        out = p.stdout.split()
        print(json.dumps(dict(name=r['name'], sizes=s, budget=b, result=out[0] if out else 'ERR',
                              nodes=out[1] if len(out) > 1 else None, sec=round(time.time() - t0, 3),
                              rc=p.returncode)), flush=True)
