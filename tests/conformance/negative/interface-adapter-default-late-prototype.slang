interface I{int read(float x);}struct V:I{int read(int x,int y);int read(int x,int y=7){return x+y;}};[shader("compute")][numthreads(1,1,1)]void main(){}
