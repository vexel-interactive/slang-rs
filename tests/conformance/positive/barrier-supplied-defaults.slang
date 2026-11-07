float readShared()
{
    AllMemoryBarrierWithGroupSync();
    return 1;
}

float sample(float value = readShared()) { return value; }
struct Packet { float value = readShared(); };

[shader("fragment")]
float4 main() : SV_Target
{
    Packet packet = Packet(2);
    return float4(sample(3) + packet.value);
}
