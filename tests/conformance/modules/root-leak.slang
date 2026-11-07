import root_leak_lib;
static float value = 3.0;
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = read();
}
