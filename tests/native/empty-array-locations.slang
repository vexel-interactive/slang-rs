typedef float Empty[0];
Empty makeArray(inout int i) { i++; return Empty(); }
int makeIndex(inout int i) { i++; return i; }
void write(out float x) { x = 4; }
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int i = 0;
    float value = makeArray(i)[makeIndex(i)];
    Empty a = Empty();
    write(a[makeIndex(i)]);
    a[makeIndex(i)] += 2;
    result[0] = i + value;
}
