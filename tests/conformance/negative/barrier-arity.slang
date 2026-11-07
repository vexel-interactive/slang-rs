[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    DeviceMemoryBarrier(1);
}
