StructuredBuffer<int3> input;
RWStructuredBuffer<float> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    int3 signedValue = input[0];
    uint3 unsignedValue = uint3(signedValue);
    half3 halfValue = half3(signedValue);
    float3 floatValue = float3(signedValue);
    bool3 flags = signedValue != int3(0);
    uint unsignedProduct = dot(signedValue, unsignedValue);
    half halfProduct = dot(signedValue, halfValue);
    int integerProduct = dot(flags, floatValue);
    float splatProduct = dot(floatValue, 1.0);
    output[0] = float(unsignedProduct) + float(halfProduct) + float(integerProduct) + splatProduct;
}
