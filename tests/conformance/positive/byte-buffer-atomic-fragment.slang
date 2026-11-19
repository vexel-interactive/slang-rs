RWByteAddressBuffer data : register(u0);

[shader("fragment")]
float4 main() : SV_Target {
    int previous;
    data.InterlockedMin(0, -1, previous);
    data.InterlockedMax(4, 7);
    data.InterlockedCompareStore(8, 2, 3);
    return float4(float(previous), 0, 0, 1);
}
