StructuredBuffer<float> source;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    source[0] = 1.0;
}
