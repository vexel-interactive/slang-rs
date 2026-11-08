// Pointer-width arithmetic preserves sign, high bits, casts, forward dimensions and compound results.
static const intptr_t Minimum=-9223372036854775808Z;
static const uintptr_t Maximum=0xffffffffffffffffUZ;
static const uintptr_t Quotient=Maximum/3UZ;
static const uintptr_t Shift=Maximum>>63;
static const intptr_t SignedShift=Minimum>>62;
static const bool HighUnsigned=Maximum>0x7fffffffffffffffZ;
static const intptr_t Count=(1Z<<40)>>39;

RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    intptr_t values[Count]={-9Z,-7Z};intptr_t x=values[0];uint divisor=2u;
    intptr_t assigned=(x/=divisor);uintptr_t narrow=9UZ;narrow/=-2LL;
    output[0]=uint(Minimum);
    output[1]=uint(uintptr_t(Minimum)>>32);
    output[2]=uint(Maximum);
    output[3]=uint(Maximum>>32);
    output[4]=uint(Quotient);
    output[5]=uint(Quotient>>32);
    output[6]=uint(Shift);
    output[7]=uint(SignedShift);
    output[8]=uint(assigned);
    output[9]=uint(x);
    output[10]=uint(narrow);
    output[11]=uint(Count);
    output[12]=uint(intptr_t(uint(0xffffffff))>>32);
    output[13]=uint(uintptr_t(-1)>>32);
    output[14]=uint(intptr_t(1099511627776.0L)>>40);
    output[15]=uint((-0xffffffffffffffffZ));
    output[16]=uint(~0xffffffffffffffffZ);
    output[17]=uint(0xffffffffffffffffZ>>62);
    output[18]=uint((0xffffffffffffffffUZ>>62));
    output[19]=uint((1UZ<<63)>>63);
    output[20]=uint((1Z<<63)>>63);
    output[21]=uint(Maximum%7UZ);
    output[22]=uint(values[1]);
    output[23]=uint(HighUnsigned);
    output[24]=uint(Maximum>0UZ);
}
