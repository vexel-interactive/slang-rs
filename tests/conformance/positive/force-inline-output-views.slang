RWStructuredBuffer<uint> output;
[ForceInline] void shift16(inout uint16_t value) { value >>= 1; }
[ForceInline] uint64_t alias64(inout uint64_t a, inout int64_t b) { a = 9ULL; return uint64_t(b); }
[ForceInline] void shift64(inout uint64_t value) { value >>= 1; }
[ForceInline] uintptr_t aliasPointer(inout uintptr_t a, inout intptr_t b) { a = uintptr_t(11); return uintptr_t(b); }

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    int16_t small = int16_t(-2);
    shift16(small);
    output[0] = uint(small);
    int64_t large = 1LL;
    output[1] = uint(alias64(large, large));
    output[2] = uint(large);
    large = -2LL;
    shift64(large);
    output[3] = uint(large);
    output[4] = uint(large >> 32);
    intptr_t pointer = intptr_t(1);
    output[5] = uint(aliasPointer(pointer, pointer));
    output[6] = uint(pointer);
}
