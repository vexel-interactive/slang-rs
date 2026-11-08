RWStructuredBuffer<uint64_t> output;
[shader("compute")][numthreads(1,1,1)]void main(){output[0]=0x10000000000000000;}
