struct R{int value;};interface I{int read(float x);}struct V:I{R read(float x,int y=7){return{y};}int read(int x){return 11;}};
[shader("compute")][numthreads(1,1,1)]void main(){}
