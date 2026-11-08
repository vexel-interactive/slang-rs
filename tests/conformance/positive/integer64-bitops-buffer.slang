StructuredBuffer<int64_t4> input;
RWStructuredBuffer<uint64_t4> output;
[shader("compute")][numthreads(1,1,1)]void main(uint3 tid:SV_DispatchThreadID)
{
    int64_t4 value=input[tid.x];
    output[tid.x]=uint64_t4(countbits(value));
    output[tid.x+1]=uint64_t4(firstbithigh(value));
    output[tid.x+2]=uint64_t4(firstbitlow(value));
    output[tid.x+3]=(uint64_t4)reversebits(value);
}
