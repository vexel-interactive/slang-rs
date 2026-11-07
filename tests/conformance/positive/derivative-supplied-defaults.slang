float read() { return ddx(1.0); }
float sample(float value = read()) { return value; }
struct Packet { float value = read(); };

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Packet packet = Packet(2);
    float value = sample(3) + packet.value;
}
