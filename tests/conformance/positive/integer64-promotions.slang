// Mixed integer promotions must retain signed division and evaluate destinations once.
RWStructuredBuffer<uint> output;
uint nextIndex(inout uint count){uint result=count;count++;return result;}
[shader("compute")][numthreads(1,1,1)]void main()
{
    int64_t signedValue=-9LL;uint divisor=2u;
    int64_t quotient=(signedValue/=divisor);
    output[0]=uint(quotient);output[1]=uint(signedValue);
    uint narrow=9u;narrow/=int64_t(-2);output[2]=narrow;
    narrow=9u;narrow%=int64_t(-2);output[3]=narrow;
    bool select=true;uint alternative=2u;
    output[4]=uint((select?int64_t(-9):alternative)/2LL);
    output[5]=uint((select?int64_t(-9):alternative)<0LL);
    output[6]=uint(min(int64_t(-9),alternative));
    output[7]=uint(max(int64_t(-9),alternative));
    output[8]=uint(clamp(int64_t(-9),-10LL,alternative));
    int64_t2 v=int64_t2(-9LL,-7LL);int64_t2 returned=(v/=divisor);
    output[9]=uint(returned.x);output[10]=uint(v.y);
    uint2 u=uint2(9u,7u);u/=int64_t(-2);output[11]=u.x;output[12]=u.y;
    uint2x2 m=uint2x2(9u,7u,5u,3u);m/=int64_t(-2);
    output[13]=m._m00;output[14]=m._m01;output[15]=m._m10;output[16]=m._m11;
    uint values[2]={9u,7u};uint counter=0;
    output[17]=(values[nextIndex(counter)]/=int64_t(-2));
    output[18]=counter;output[19]=values[0];output[20]=values[1];
    int64_t changed=1LL;changed+=alternative;changed*=alternative;changed-=1u;
    changed|=8u;changed^=1u;changed&=15u;output[21]=uint(changed);
    int signed32=9;signed32/=2.5;output[22]=uint(signed32);
    uint bitwise32=9u;bitwise32&=2.5;output[23]=bitwise32;
}
