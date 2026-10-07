// Authored bounded external-reader experiment, not a translated parser.
// Field ordering and pinned provenance: docs/plan-prefix.md.
import com.google.gson.*;
import cwlib.types.SerializedResource;
import cwlib.io.streams.MemoryInputStream;
import java.io.*;
import java.nio.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

public class PlanOracle {
    static final String PIN="d6a2ef80b5a09999f08f23a803b516096055e5c9";
    static final int EXPANDED=16*1024*1024;
    static void need(byte[] b,int at,int n)throws IOException {
        if(at<0||n<0||at>b.length-n)throw new IOException("bounds");
    }
    // Strict guard before calling community i32: five-byte unsigned integer only.
    static int integerEnd(byte[] b,int at,byte flags)throws IOException {
        if(flags==0){need(b,at,4);return at+4;}
        if(flags!=7)throw new IOException("unsupported flags");
        for(int i=0;i<5;i++) {
            need(b,at+i,1);int v=b[at+i]&255;
            if(i==4&&v>15)throw new IOException("integer overflow");
            if((v&128)==0)return at+i+1;
        }
        throw new IOException("integer overflow");
    }
    static long integer(MemoryInputStream s,byte flags)throws IOException {
        int end=integerEnd(s.getBuffer(),s.getOffset(),flags);
        long value=Integer.toUnsignedLong(s.i32());
        if(s.getOffset()!=end)throw new IOException("reader integer offset disagreement");
        return value;
    }
    static Map<String,Object> prefix(byte[] clear,int revision,byte flags)throws Exception {
        if(revision<893||revision>1016||clear.length>EXPANDED)throw new IOException("unsupported revision or budget");
        MemoryInputStream s=new MemoryInputStream(clear,flags);
        long inner=integer(s,flags);int lengthOffset=s.getOffset();
        long size=integer(s,flags);int start=s.getOffset();
        if(size>EXPANDED||size>clear.length-start)throw new IOException("opaque array budget/bounds");
        // Exercise unmodified reader bytearray only after its count/extent are bounded.
        s.seek(lengthOffset,MemoryInputStream.SeekMode.Begin);byte[] things=s.bytearray();
        if(things.length!=size||s.getOffset()!=start+size)throw new IOException("reader bytearray disagreement");
        return TextureOracle.row("revision",revision,"inner_revision_raw_u32",inner,
            "thing_data_bytes",size,"thing_data_start",start,"thing_data_sha256",TextureOracle.sha(things),
            "parsed_prefix_bytes",start,"unparsed_bytes",clear.length-start,
            "inventory_tail_start",s.getOffset(),"inventory_tail_bytes",clear.length-s.getOffset());
    }
    static int dependencies(byte[] raw,int at)throws IOException {
        need(raw,at,4);int count=ByteBuffer.wrap(raw).getInt(at);
        if(count<0||count>4096)throw new IOException("dependency count budget");
        int p=at+4;
        for(int i=0;i<count;i++) {
            need(raw,p,1);int flags=raw[p++]&255;
            if(flags>3)throw new IOException("unsupported dependency flags");
            int bytes=((flags&2)!=0?4:0)+((flags&1)!=0?20:0)+4;
            need(raw,p,bytes);p+=bytes;
        }
        if(p!=raw.length)throw new IOException("dependency trailing data outside scope");
        return count;
    }
    static Map<String,Object> inspect(byte[] raw)throws Exception {
        need(raw,0,18);ByteBuffer h=ByteBuffer.wrap(raw);
        if(h.getInt()!=0x504c4e62)throw new IOException("only ordinary clear PLNb; protected input blocked");
        int revision=h.getInt(),end=h.getInt(),branch=h.getInt();
        byte flags=h.get();int compressed=h.get()&255;
        if(raw.length>2*1024*1024||revision<893||revision>1016||branch!=0||
                (flags!=0&&flags!=7)||compressed>1||end<18||end>raw.length-4)
            throw new IOException("unsupported header/revision or stored budget");
        int count=dependencies(raw,end);
        byte[] clear=Arrays.copyOfRange(raw,18,end);
        if(compressed!=0) {
            need(clear,0,4);if(TextureOracle.be16(clear,0)>1)throw new IOException("unsupported chunk flag");
            int chunks=TextureOracle.be16(clear,2);need(clear,4,4*chunks);
            long expanded=0;
            if(chunks==0)expanded=clear.length-4;
            for(int i=0;i<chunks;i++)expanded+=TextureOracle.be16(clear,6+4*i);
            if(expanded>EXPANDED)throw new IOException("expanded budget before inflation");
            byte[] wrapped=new byte[clear.length+4];System.arraycopy(new byte[]{'T','E','X',' '},0,wrapped,0,4);
            System.arraycopy(clear,0,wrapped,4,clear.length);clear=TextureOracle.clearPayload(wrapped);
        }
        if(clear.length>EXPANDED)throw new IOException("expanded budget");
        // Constructor cannot encounter encrypted branch or unbounded table/inflation.
        SerializedResource resource=new SerializedResource(raw);
        if(resource.getCompressionFlags()!=flags||!Arrays.equals(clear,resource.getStream().getBuffer()))
            throw new IOException("wrapper disagreement");
        return TextureOracle.row("compression_flags",Byte.toUnsignedInt(flags),"expanded_bytes",clear.length,
            "dependency_count_raw",count,"reader_unique_dependencies",resource.getDependencies().length,
            "prefix",prefix(clear,revision,flags));
    }
    static void encoded(ByteArrayOutputStream b,long value,byte flags)throws IOException {
        if(flags==0){new DataOutputStream(b).writeInt((int)value);return;}
        do{int v=(int)(value&127);value>>>=7;b.write(v|(value!=0?128:0));}while(value!=0);
    }
    static byte[] synthetic(byte flags,boolean compressed)throws Exception {
        ByteArrayOutputStream p=new ByteArrayOutputStream();encoded(p,0xffffffffL,flags);encoded(p,3,flags);
        p.write(new byte[]{1,2,3});p.write(new byte[]{4,5});byte[] clear=p.toByteArray();
        ByteArrayOutputStream b=new ByteArrayOutputStream();DataOutputStream d=new DataOutputStream(b);
        d.writeBytes("PLNb");d.writeInt(919);d.writeInt(18+clear.length+(compressed?4:0));
        d.writeInt(0);d.writeByte(flags);d.writeBoolean(compressed);
        if(compressed)d.writeInt(0); // zero-chunk stored envelope
        d.write(clear);d.writeInt(2);
        d.writeByte(2);d.writeInt(42);d.writeInt(1);
        d.writeByte(2);d.writeInt(42);d.writeInt(1); // duplicate table entries deliberately
        return b.toByteArray();
    }
    static void rejected(byte[] raw)throws Exception {
        try{inspect(raw);}catch(IOException expected){return;}
        throw new AssertionError("accepted malformed/unsupported wrapper");
    }
    static void selfTest()throws Exception {
        for(byte flags:new byte[]{0,7})for(boolean compressed:new boolean[]{false,true}) {
            Map<String,Object> r=inspect(synthetic(flags,compressed));
            Map<?,?> p=(Map<?,?>)r.get("prefix");
            if(!p.get("inner_revision_raw_u32").equals(0xffffffffL)||!p.get("thing_data_bytes").equals(3L)
                ||!p.get("inventory_tail_bytes").equals(2)||!r.get("dependency_count_raw").equals(2)
                ||!r.get("reader_unique_dependencies").equals(1))throw new AssertionError("synthetic metadata");
            byte[] raw=synthetic(flags,compressed);
            for(int n=0;n<raw.length;n++)rejected(Arrays.copyOf(raw,n));
            raw[3]='e';rejected(raw);raw=synthetic(flags,compressed);raw[12]=1;rejected(raw);
        }
        for(byte flags:new byte[]{0,7}) {
            ByteArrayOutputStream b=new ByteArrayOutputStream();encoded(b,919,flags);encoded(b,EXPANDED+1L,flags);
            try{prefix(b.toByteArray(),919,flags);throw new AssertionError("array budget");}catch(IOException expected){}
        }
        try{prefix(new byte[]{(byte)255,(byte)255,(byte)255,(byte)255,16},919,(byte)7);
            throw new AssertionError("integer overflow");}catch(IOException expected){}
        byte[] raw=synthetic((byte)7,true);
        for(int revision:new int[]{892,1017}) {
            byte[] bad=raw.clone();ByteBuffer.wrap(bad).putInt(4,revision);rejected(bad);
        }
        byte[] bad=raw.clone();bad[16]=1;rejected(bad);
        int end=ByteBuffer.wrap(raw).getInt(8);
        bad=raw.clone();ByteBuffer.wrap(bad).putInt(end,4097);rejected(bad);
        bad=raw.clone();bad[end+4]=8;rejected(bad);
        rejected(Arrays.copyOf(raw,raw.length+1));
        // Declared chunk expansion exceeds budget, with tiny stored input. Rejected
        // before either strict inflater or the community resource constructor runs.
        int chunks=257,bytes=4+4*chunks;
        ByteBuffer large=ByteBuffer.allocate(18+bytes+4);
        large.putInt(0x504c4e62).putInt(919).putInt(18+bytes).putInt(0).put((byte)7).put((byte)1);
        large.putShort((short)0).putShort((short)chunks);
        for(int i=0;i<chunks;i++)large.putShort((short)0).putShort((short)65535);
        large.putInt(0);rejected(large.array());
        System.out.println("Plan synthetic prefix/bounds/flags/wrappers/dependencies guards passed");
    }
    public static void main(String[] args)throws Exception {
        if(args.length==1&&args[0].equals("--self-test")){selfTest();return;}
        if(args.length!=3)throw new IllegalArgumentException("FARC MANIFEST NEW-JSON");
        Path output=Path.of(args[2]).toAbsolutePath();
        if(!output.toString().endsWith(".json")||Files.exists(output)||
            !output.getParent().toRealPath().startsWith(Path.of("local-analysis").toRealPath()))throw new IOException("new ignored JSON required");
        JsonObject manifest=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject();
        if(!manifest.get("toolkit_commit").getAsString().equals(PIN)||
            !manifest.get("scope").getAsString().equals("clear_plan_prefix_revision_893_951_branch_zero"))throw new IOException("manifest pin/scope differs");
        JsonArray samples=manifest.getAsJsonArray("samples");if(samples.size()>48)throw new IOException("sample budget");
        List<Object> rows=new ArrayList<>();long total=0,deadline=System.nanoTime()+60_000_000_000L;
        try(RandomAccessFile f=new RandomAccessFile(args[0],"r")) {
            for(JsonElement item:samples) {
                if(Thread.currentThread().isInterrupted()||System.nanoTime()>deadline)throw new IOException("oracle cancelled/time budget");
                JsonObject q=item.getAsJsonObject();String sha=q.get("sha1").getAsString();
                Map<String,Object> r=TextureOracle.row("sha1",sha);rows.add(r);
                try {
                    long at=q.get("offset").getAsLong(),size=q.get("size").getAsLong();
                    if(!TextureOracle.sameArchive(q.get("source_path").getAsString(),args[0])||at<0||size<18||size>2*1024*1024||
                        at>f.length()-size||total+size>64*1024*1024)throw new IOException("source/read budget");
                    byte[] raw=new byte[(int)size];f.seek(at);f.readFully(raw);total+=size;
                    if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-1").digest(raw)).equals(sha))throw new IOException("identifier mismatch");
                    r.putAll(inspect(raw));r.put("status","parsed_prefix");
                }catch(Exception e){r.putAll(TextureOracle.row("status","rejected","reason",e.getClass().getSimpleName()+": "+e.getMessage()));}
            }
        }
        Files.writeString(output,new GsonBuilder().serializeNulls().setPrettyPrinting().create().toJson(TextureOracle.row(
            "schema_version",1,"toolkit_commit",PIN,"scope","plan_prefix_metadata_no_thing_traversal",
            "sample_bytes",total,"samples",rows)),StandardOpenOption.CREATE_NEW);
        System.out.println("Plan oracle: "+rows.size()+" samples, "+total+" archived bytes");
    }
}
