RWStructuredBuffer<float> output;

float kind(half value) { return 1; }
float kind(float value) { return 2; }
float kind(double value) { return 3; }

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = kind(1.5h) + kind(1.5H) + kind(1.5hf) + kind(1.5HF)
        + kind(1.5fh) + kind(1.5FH);
    output[1] = kind(1.5) + kind(1.5f) + kind(1.5F);
    output[2] = kind(1.5l) + kind(1.5L) + kind(1.5lf) + kind(1.5LF)
        + kind(1.5fl) + kind(1.5FL);
    output[3] = kind(0x1.8p0hf) + kind(0x1.8p0f) + kind(0x1.8p0lf);
    output[4] = kind(1.e0hf) + kind(.5e+1f) + kind(1.e-1LF);
}
