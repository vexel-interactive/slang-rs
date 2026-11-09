struct S{int value;__init(int x);__init(int x){value=x;}};[shader("compute")][numthreads(1,1,1)]void main(){S x=S(2);}
