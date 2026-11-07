[shader("fragment")]
float4 main(float2 uv : TEXCOORD0) : SV_Target
{
    int value = int(uv.x);
    uint2 vectorValue = uint2(uv);
    int2x3 matrixValue = int2x3(value);
    float scalarResult = ddx(value);
    float2 vectorResult = ddy(vectorValue);
    float2x3 matrixResult = fwidth_fine(matrixValue);
    return float4(scalarResult + vectorResult.x + matrixResult[0][0]);
}
