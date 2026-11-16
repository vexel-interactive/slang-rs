func invalid() -> float4 : SV_Target
{
    return float4(1);
}

[shader("compute")]
[numthreads(1, 1, 1)]
func main()
{
}
