typedef float Row[2];
typealias Grid = Row[3];
typedef int Trailing[2][3];
typedef int[2][3] Leading;

RWStructuredBuffer<float> result;

float first(Grid values)
{
    return values[0][0];
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Grid grid;
    Trailing a;
    Leading b;
    grid[0][0] = 1;
    a[1][2] = 2;
    b[2][1] = 3;
    result[0] = first(grid) + float(a[1][2] + b[2][1]);
}
