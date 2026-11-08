RWStructuredBuffer<float> output;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float a[int(1.99999999)] = {1, 2};
    float b[int(1.99999999lf)] = {3};
    float c[int(2.9999hf)] = {4, 5, 6};
    float d[int(-3.5) + 5] = {7, 8};
    float e[float(2.5)] = {9, 10};
    float f[- -2.5] = {11, 12};
    float g[2L] = {13, 14};
    output[0] = a[1] + b[0] + c[2] + d[1] + e[1] + f[1] + g[1];
}
