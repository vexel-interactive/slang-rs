#define LETTER '\x41'
static const uint Count = Last - LETTER;
static const uint Last = 'C';
RWStructuredBuffer<uint> output;

uint choose(int value) { return 1; }
uint choose(uint value) { return 2; }
uint pick(uint value = '\u0041') { return value; }
uint select(uint value)
{
    switch (value)
    {
        case 'A': return '\777';
        case 'B': return '\Uffffffff';
        default: return 0;
    }
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    uint values[Count] = {LETTER, 'B'};
    output[0] = choose('A');
    output[1] = pick();
    output[2] = select(values[0]);
    output[3] = select(values[1]);
}
