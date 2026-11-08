RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = 1e50f;
    output[1] = 65520.0hf;
    output[2] = 1e-100f;
    output[3] = 0x1p-150f;
    output[4] = float(1e9999LF);
}
