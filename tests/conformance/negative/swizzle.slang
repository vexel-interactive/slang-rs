[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2 value = float2(1.0, 2.0);
    value.z = 3.0;
}
