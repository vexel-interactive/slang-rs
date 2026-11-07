groupshared uint value;

[shader("fragment")]
float4 main() : SV_Target
{
    uint value = 2u;
    return float4(value);
}
