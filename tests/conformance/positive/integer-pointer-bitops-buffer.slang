StructuredBuffer<vector<intptr_t,4>> input;
RWStructuredBuffer<vector<uintptr_t,4>> output;
[shader("compute")][numthreads(1,1,1)]void main(uint3 tid:SV_DispatchThreadID)
{
    vector<intptr_t,4> value=input[tid.x];
    output[tid.x]=vector<uintptr_t,4>(countbits(value));
    output[tid.x+1]=vector<uintptr_t,4>(firstbithigh(value));
    output[tid.x+2]=vector<uintptr_t,4>(firstbitlow(value));
    output[tid.x+3]=(vector<uintptr_t,4>)reversebits(value);
}
