groupshared uintptr_t value;
[shader("compute")][numthreads(1,1,1)]void main(){InterlockedAdd(value,1UZ);}
