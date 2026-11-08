RWStructuredBuffer<uint64_t> output;
[shader("compute")][numthreads(1,1,1)]void main(){output[0]=18446744073709551616ULL;}
