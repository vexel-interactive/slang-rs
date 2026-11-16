import library;

static const int hidden = 100;
RWStructuredBuffer<uint> output;

[shader("compute")]
[numthreads(1, 1, 1)]
func main()
{
    output[0] = uint(adjusted());
    output[1] = uint(copy(4));
}
