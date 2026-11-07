[shader("vertex")]
float4 main() : SV_Position
{
    clip(-1.0);
    return float4(0.0);
}
