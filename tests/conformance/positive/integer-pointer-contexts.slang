// Aliases, defaults, computed shapes, source prefixes and writable pointer locations.
#define GROUP_SIZE 1Z
#if GROUP_SIZE != 1
#error Pointer suffix was not consumed in preprocessing
#endif
typedef vector<intptr_t,2> Pair;
static const uintptr_t Count=1UZ;
Pair read(Pair value=Pair(1Z,2Z)){return value;}
uint classify(intptr_t value){switch(value){case 0xffffffffffffffffZ:return 1;case 0xffffffff:return 2;default:return 3;}}
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(Count,GROUP_SIZE,1ZU)]void main()
{
    Pair local=vector<intptr_t,2>(3Z);Pair value=read();
    intptr_t values[2]={9Z,7Z};uintptr_t index=0UZ;
    intptr_t result=(values[index++]/=2u);
    output[0]=uint(local.x+value.y);
    output[1]=uint(result);output[2]=uint(index);output[3]=uint(values[1]);
    output[4]=classify(-1Z);output[5]=classify(0xffffffff);output[6]=classify(0x100000000Z);
    Pair reversed=reversebits(Pair(1Z,2Z));
    output[7]=uint(uintptr_t(reversed.x)>>32);output[8]=uint(uintptr_t(reversed.y)>>32);
    output[9]=uint(value.x);
}
