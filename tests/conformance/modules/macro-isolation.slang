#define ROOT_ONLY 3
import macro_lib;
#ifdef IMPORT_ONLY
#error import macro leaked
#endif
float run(){return macroRead()+ROOT_ONLY;}
RWStructuredBuffer<float> result;
[shader("compute")][numthreads(1,1,1)]void main(){result[0]=run();}
