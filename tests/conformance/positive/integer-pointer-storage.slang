// Pointer integers retain source identities while HLSL uses 64-bit storage.
typedef vector<intptr_t,2> Pair;
struct Params { float lead; intptr_t signedValue; vector<uintptr_t,3> vectorValue; column_major matrix<uintptr_t,2,3> matrixValue; uintptr_t values[2]; };
ConstantBuffer<Params> params;
StructuredBuffer<Pair> input;
RWStructuredBuffer<uintptr_t> output;
intptr_t choose(intptr_t value){return value+1Z;}
uintptr_t choose(uintptr_t value){return value+2UZ;}
uint classify(uintptr_t value){switch(value){case 0xffffffffffffffffUZ:return 1;case 0x100000000UZ:return 2;default:return 3;}}
[shader("compute")][numthreads(1,1,1)]void main(uint3 tid:SV_DispatchThreadID)
{
    Pair a=input[tid.x];
    vector<intptr_t,2> b=vector<intptr_t,2>(a.x,params.signedValue);
    vector<uintptr_t,2> c=(vector<uintptr_t,2>)(a+b);
    matrix<uintptr_t,2,3> m=matrix<uintptr_t,2,3>(c.x,c.y,params.vectorValue.x,params.vectorValue.y,params.values[0],params.values[1]);
    output[tid.x]=choose(c.x)+(m+params.matrixValue)._m12+uintptr_t(classify(c.y));
    output[tid.x+1]=uintptr_t(choose(a.y));
    output[tid.x+2]=uintptr_t(dot(a,b));
    output[tid.x+3]=uintptr_t(abs(a.x))+min(c.x,c.y)+max(c.x,c.y)+clamp(c.x,0UZ,c.y);
}
