RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float[2] row = {1.0, 2.0}, grid[3] = {{3.0, 4.0}, {5.0, 6.0}, {7.0, 8.0}};
    float sum = 0.0;
    for (float a[2] = {1.0, 2.0}, b = a[1]; b < 3.0; ++b)
    {
        sum += a[0] + b;
    }
    result[0] = row[0] + grid[2][1] + sum;
}
