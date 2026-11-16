StructuredBuffer<float4> input;
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    float4 source = input[id.x];
    float2x2 matrixSource = float2x2(source.x, source.y, source.z, source.w);

    int16_t narrowSigned = (int16_t) source.x;
    uint16_t narrowUnsigned = (uint16_t) source.y;
    int16_t2 narrowPair = (int16_t2) source.xy;
    uint16_t2x2 narrowMatrix = (uint16_t2x2) matrixSource;
    float16_t narrowFloat = (float16_t) source.z;
    float16_t3 narrowTriple = (float16_t3) source.xyz;
    float16_t2x2 narrowFloatMatrix = (float16_t2x2) matrixSource;

    int32_t2 signedPair = (int32_t2) source.xy;
    uint32_t3 unsignedTriple = (uint32_t3) source.xyz;
    int32_t2x2 signedMatrix = (int32_t2x2) matrixSource;
    float32_t4 floatQuad = (float32_t4) source;
    float32_t2x2 floatMatrix = (float32_t2x2) matrixSource;

    int64_t3 wideTriple = (int64_t3) source.xyz;
    uint64_t2x2 wideMatrix = (uint64_t2x2) matrixSource;
    float64_t2 doublePair = (float64_t2) source.xy;
    float64_t2x2 doubleMatrix = (float64_t2x2) matrixSource;
    intptr_t signedPointer = (intptr_t) source.x;
    uintptr_t unsignedPointer = (uintptr_t) source.y;

    output[0] = uint(narrowSigned) + uint(narrowUnsigned);
    output[1] = uint(narrowPair.y) + uint(narrowMatrix[1][0]);
    output[2] = asuint(narrowFloat) + asuint(narrowTriple.z);
    output[3] = asuint(narrowFloatMatrix[0][1]);
    output[4] = uint(signedPair.x) + unsignedTriple.z;
    output[5] = uint(signedMatrix[1][1]);
    output[6] = asuint(floatQuad.w) + asuint(floatMatrix[1][0]);
    output[7] = uint(wideTriple.z) + uint(wideMatrix[0][1]);
    output[8] = uint(doublePair.y) + uint(doubleMatrix[1][1]);
    output[9] = uint(signedPointer) + uint(unsignedPointer);
    output[10] = uint((ssize_t) source.z) + uint((size_t) source.w);
    output[11] = uint((usize_t) source.x);
}
