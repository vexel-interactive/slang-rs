RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = 0#INF;
    output[1] = -1#INFf;
    output[2] = 1.0#INFhf;
    output[3] = float(-1#INFLF);
    output[4] = 0e+5 + 0E-3;
}
