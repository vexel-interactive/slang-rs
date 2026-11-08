// Dynamic wide values exercise shapes, constructors, overloads, buffers and switches.
typedef vector<int64_t,2> Pair;
struct Params { float lead; int64_t signedValue; uint64_t3 vectorValue; column_major uint64_t2x3 matrixValue; uint64_t values[2]; };
ConstantBuffer<Params> params;
StructuredBuffer<Pair> input;
RWStructuredBuffer<uint64_t> output;
int64_t choose(int64_t value){return value+1LL;}
uint64_t choose(uint64_t value){return value+2ULL;}
uint classify(uint64_t value){switch(value){case 0xffffffffffffffffULL:return 1;case 0x100000000ULL:return 2;default:return 3;}}
[shader("compute")][numthreads(1,1,1)]void main(uint3 tid:SV_DispatchThreadID)
{
    Pair a=input[tid.x];
    int64_t2 b=int64_t2(a.x,params.signedValue);
    uint64_t2 c=(uint64_t2)(a+b);
    uint64_t2x3 m=uint64_t2x3(c.x,c.y,params.vectorValue.x,params.vectorValue.y,params.values[0],params.values[1]);
    output[tid.x]=choose(c.x)+(m+params.matrixValue)._m12+uint64_t(classify(c.y));
    output[tid.x+1]=uint64_t(choose(a.y));
    output[tid.x+2]=uint64_t(dot(a,b));
    output[tid.x+3]=uint64_t(abs(a.x))+min(c.x,c.y)+max(c.x,c.y)+clamp(c.x,0ULL,c.y);
}
