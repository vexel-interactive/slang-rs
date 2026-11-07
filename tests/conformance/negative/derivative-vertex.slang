float read() { return ddy_fine(1.0); }

[shader("vertex")]
float4 main() : SV_Position
{
    return float4(read());
}
