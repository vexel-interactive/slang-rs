float read(float value, float bias = 3.0) { return value + bias; }
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int x = 0;
    int y = 1;
    for (x = 0, y = 1; x < 3; ++x, ++y)
    {
        result[x] = read((y += 2, float(y)), float(x));
    }
    result[3] = (x = 4, read(float(x)));
}
