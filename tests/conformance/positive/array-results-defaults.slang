typedef float Values[2];
typedef float Grid[2][3];
Values make(inout int count) { count++; return Values(count, count + 1); }
Values defaults() { return {1, 2}; }
static Values initial = defaults();
Values choose(Values values = defaults()) { return values; }
Grid choose(Grid values) { return values; }
RWStructuredBuffer<float> result;

[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    int count = 0;
    Values a = make(count);
    Values b = choose();
    Grid grid = choose(float[3][2](1, 2, 3, 4, 5, 6));
    result[0] = count + a[1] + b[1] + grid[1][2] + initial[1];
}
