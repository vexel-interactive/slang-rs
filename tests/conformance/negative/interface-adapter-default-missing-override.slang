interface I{int read(float x=3.9){return 99;}}struct V:I{int read(int x,int y=37){return x+y;}};[shader("compute")][numthreads(1,1,1)]void main(){}
