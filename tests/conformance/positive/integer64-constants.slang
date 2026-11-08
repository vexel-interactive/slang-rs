// Compiled words verify signed and unsigned 64-bit arithmetic, conversions and shapes.
static const uint64_t Max=0xffffffffffffffffULL;
static const int64_t Min=-9223372036854775808;
static const int64_t Signed=int64_t(uint(0xffffffff));
static const uint64_t Wrapped=uint64_t(-1);
static const uint64_t Quotient=Max/3ULL;
static const uint64_t Remainder=Max%7ULL;
static const uint64_t Product=(1ULL<<40)*3ULL;
static const uint64_t Negated=-0x8000000000000001ULL;
static const uint64_t Sum=Max+2ULL;
static const uint64_t Difference=0ULL-2ULL;
static const uint64_t Complement=~0ULL;
static const uint64_t Bits=(0xf000000000000000ULL|0x0100000000000000ULL)^0x8000000000000000ULL;
static const uint64_t Shift=Max>>63;
static const int64_t SignedShift=Min>>62;
static const bool Compare=Max>0x7fffffffffffffffLL;
static const uint64_t Selected=Compare?Max:0ULL;
static const int64_t Count=(1LL<<40)>>39;
float data[Count];
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(int(Count)-1,1,1)]void main()
{
    output[0]=uint(Max);output[1]=uint(Max>>32);
    output[2]=uint(Min);output[3]=uint(Min>>32);
    output[4]=uint(Signed);output[5]=uint(Signed>>32);
    output[6]=uint(Wrapped);output[7]=uint(Wrapped>>32);
    output[8]=uint(Quotient);output[9]=uint(Quotient>>32);
    output[10]=uint(Remainder);output[11]=uint(Remainder>>32);
    output[12]=uint(Product);output[13]=uint(Product>>32);
    output[14]=uint(Negated);output[15]=uint(Negated>>32);
    output[16]=uint(Sum);output[17]=uint(Sum>>32);
    output[18]=uint(Difference);output[19]=uint(Difference>>32);
    output[20]=uint(Complement);output[21]=uint(Complement>>32);
    output[22]=uint(Bits);output[23]=uint(Bits>>32);
    output[24]=uint(Shift);output[25]=uint(Shift>>32);
    output[26]=uint(SignedShift);output[27]=uint(SignedShift>>32);
    output[28]=uint(Selected);output[29]=uint(Selected>>32);
    output[30]=uint(int64_t(1099511627776.0L));output[31]=uint(int64_t(1099511627776.0L)>>32);
    output[32]=uint(Max>0ULL);
    output[33]=uint(Max<0ULL);
    output[34]=uint(Max>=Wrapped);
    output[35]=uint(Max<=0x7fffffffffffffffLL);
    output[36]=uint(Max!=0ULL);
    output[37]=uint(Max==Wrapped);
    output[38]=uint(int64_t(-1)+uint(1));
    output[39]=uint(int64_t(-1)<uint(1));
}
