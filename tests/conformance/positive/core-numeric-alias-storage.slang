// Core aliases share canonical constructors, buffer types, overloads and legacy HLSL layout.
typedef float16_t2 HalfPair;
struct Params{float16_t halfValue;float32_t3 vectorValue;float64_t doubleValue;int32_t2x2 matrixValue;uint32_t count;};
ConstantBuffer<Params> params;
StructuredBuffer<HalfPair> input;
RWStructuredBuffer<float64_t> output;
[shader("compute")][numthreads(1,1,1)]void main(uint3 tid:SV_DispatchThreadID)
{
    HalfPair pair=input[tid.x];
    float32_t3 value=float32_t3(pair.x,pair.y,params.halfValue);
    int32_t2x2 matrixValue=int32_t2x2(1,2,3,4);
    output[tid.x]=float64_t(dot(value,params.vectorValue))+params.doubleValue+float64_t((matrixValue+params.matrixValue)._m11)+float64_t(params.count);
}
