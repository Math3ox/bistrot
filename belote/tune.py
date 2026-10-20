import subprocess, sys, itertools, os
def run(np_, games, seed, l0, l1, a):
    subprocess.run(['./belote.exe','-sim',str(games),str(seed),str(np_),str(l0),str(l1),'0']+[str(x) for x in a],check=True)
    last=open('sim.log').read().strip().split('\n')[-1].split()
    return 0,int(last[2]),int(last[3])
base=(50,46)
res=[]
for t1 in (38,44,50,56,62):
    for t2 in (34,40,46,52):
        w=0;l=0
        for seed in (1,2,3):
            g,a,b=run(4,400,seed,1,2,[t1,t2,base[0],base[1]])
            w+=a;l+=b
        res.append((w/(w+l),t1,t2))
        print(t1,t2,round(w/(w+l),3),flush=True)
print(sorted(res,reverse=True)[:5])
