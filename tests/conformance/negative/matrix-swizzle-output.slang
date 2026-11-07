void write(out float2 value) { value = float2(2); }
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float2x3 matrixValue = float2x3(1);
    write(matrixValue._11_11);
}
