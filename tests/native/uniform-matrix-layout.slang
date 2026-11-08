struct Parameters
{
    float1x1 mfloat11;
    float1x2 mfloat12;
    float1x3 mfloat13;
    float1x4 mfloat14;
    float2x1 mfloat21;
    float2x2 mfloat22;
    float2x3 mfloat23;
    float2x4 mfloat24;
    float3x1 mfloat31;
    float3x2 mfloat32;
    float3x3 mfloat33;
    float3x4 mfloat34;
    float4x1 mfloat41;
    float4x2 mfloat42;
    float4x3 mfloat43;
    float4x4 mfloat44;
    double1x1 mdouble11;
    double1x2 mdouble12;
    double1x3 mdouble13;
    double1x4 mdouble14;
    double2x1 mdouble21;
    double2x2 mdouble22;
    double2x3 mdouble23;
    double2x4 mdouble24;
    double3x1 mdouble31;
    double3x2 mdouble32;
    double3x3 mdouble33;
    double3x4 mdouble34;
    double4x1 mdouble41;
    double4x2 mdouble42;
    double4x3 mdouble43;
    double4x4 mdouble44;
    half1x1 mhalf11;
    half1x2 mhalf12;
    half1x3 mhalf13;
    half1x4 mhalf14;
    half2x1 mhalf21;
    half2x2 mhalf22;
    half2x3 mhalf23;
    half2x4 mhalf24;
    half3x1 mhalf31;
    half3x2 mhalf32;
    half3x3 mhalf33;
    half3x4 mhalf34;
    half4x1 mhalf41;
    half4x2 mhalf42;
    half4x3 mhalf43;
    half4x4 mhalf44;
    float tail;
};

ConstantBuffer<Parameters> parameters;
RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = parameters.tail;
}
