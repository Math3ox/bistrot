import subprocess, sys
def run(games, seed, args):
    subprocess.run(['./belote.exe','-sim',str(games),str(seed),'4','2','1','0']+[str(a) for a in args],check=True)
    last=open('sim.log').read().strip().split('\n')[-1].split()
    return int(last[2]),int(last[3])
def rate(args,games=250,seeds=(11,12)):
    w=l=0
    for s in seeds:
        a,b=run(games,s,args); w+=a; l+=b
    return w/(w+l)
# args : thr1_1 thr2_1 thr1_2 thr2_2 mcn mcnb mcthr
base=[44,42,44,42]
for mcn in (20,60,200):
    print('mcn',mcn, round(rate(base+[mcn,40,12]),3),flush=True)
for thr in (-10,0,12,25):
    print('mcthr',thr, round(rate(base+[60,40,thr]),3),flush=True)
for nb in (15,40,120):
    print('mcnb',nb, round(rate(base+[60,nb,12]),3),flush=True)
