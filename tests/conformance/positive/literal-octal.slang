#if 010 != 10 || 077 != 77 || 0x10 != 16 || 0b10 != 0
#error numeric base mismatch
#endif
#if 0X10 != 0 || 4294967295u != -1 || (2147483647 + 1) != -2147483648
#error preprocessor integer width mismatch
#endif

RWStructuredBuffer<int> output;
static const int Count = 010;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int values[Count] = int[Count](010, 077, 0, 1, 2, 3, 4, 5);
    output[0] = values[0] + values[1];
}
