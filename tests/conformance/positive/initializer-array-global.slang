typedef float Row[3];
static Row values = {1.0};
static float grid[2][3] = {{1.0}, {2.0, 3.0, 4.0}};
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = values[0] + values[2] + grid[1][2];
}
