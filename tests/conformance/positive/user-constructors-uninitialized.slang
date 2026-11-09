struct Item { int value; __init(){value=8;} };
struct Incomplete { int a,b; __init(int x){a=x;} };
struct Grid { Item cells[2]; __init() {} };
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1,1,1)]void main(){Incomplete x=Incomplete(2);Item values[2];Grid grid;output[0]=uint(x.b);output[1]=uint(values[0].value);output[2]=uint(grid.cells[0].value);}
