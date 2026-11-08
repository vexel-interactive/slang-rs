void integerOutput(inout int value) { value += 1; }
void wideOutput(out int64_t value) { value = 2LL; }
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    float f = 1.5;
    integerOutput(f);
    int i = 1;
    wideOutput(i);
}
