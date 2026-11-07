static const int first = 1, second = first + 1;

struct Pair
{
    float x = 2.0, y = 3.0, values[2] = {4.0, 5.0};
};

RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float a[2] = {1.0, 2.0}, b = a[1] + 3.0, c[3] = {b, 5.0, 6.0};
    Pair pair = Pair(), copied = Pair(pair);
    float sum = 0.0;

    for (int i = 0, j = i + second; i < 3; ++i, ++j)
    {
        sum += float(j);
    }

    result[0] = sum + copied.y + pair.values[1] + a[0] + b + c[2];
}
