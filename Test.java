
public class Test {
    static int firstStatic;
    int secondInstance;
    public void incr(){
        int thirdLocal = firstStatic++;
        secondInstance++;
        thirdLocal++;
        System.out.println(firstStatic+" "+secondInstance+" "+thirdLocal);
    }
    public static void main(String args[]) {
        Test test = new Test();
        test.incr();
        firstStatic++;
        test = new Test();
        test.incr();
    }
}
