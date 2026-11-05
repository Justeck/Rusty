package dev.justeck.rusty.network;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.nio.file.*;
import java.util.*;
import java.util.function.BiPredicate;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerPlayer;
import org.slf4j.LoggerFactory;

/** Optional dedicated-server consent. Default empty grants never enable a tool. */
public final class ServerConsent {
  private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
  private static int ticks;
  private static MinecraftServer owner;

  private record Delivery(ServerPlayer player, int mask) {}

  private static final Map<UUID, Delivery> SENT = new HashMap<>();
  private static Rules rules = new Rules();
  private static long modified = Long.MIN_VALUE;

  public static final class Rules {
    public int schema = 1;
    public Set<String> everyone = new LinkedHashSet<>();
    public Map<String, Set<String>> players = new LinkedHashMap<>();
  }

  public static void tick(MinecraftServer server, BiPredicate<ServerPlayer, ConsentPayload> send) {
    if (owner != server) {
      owner = server;
      SENT.clear();
      modified = Long.MIN_VALUE;
      ticks = 0;
    }
    if (++ticks % 20 != 0) return;
    Path path = server.getServerDirectory().resolve("config/rusty-server-permissions.json");
    try {
      if (!Files.exists(path)) {
        Files.createDirectories(path.getParent());
        Files.writeString(path, GSON.toJson(new Rules()));
      }
      long stamp = Files.getLastModifiedTime(path).toMillis();
      if (stamp != modified) {
        if (Files.size(path) > 65536)
          throw new IllegalArgumentException("Permission file exceeds 64 KiB");
        Rules next = GSON.fromJson(Files.readString(path), Rules.class);
        if (next == null || next.schema != 1 || next.everyone == null || next.players == null)
          throw new IllegalArgumentException("Malformed permissions");
        rules = next;
        modified = stamp;
      }
    } catch (Exception e) {
      rules = new Rules();
      LoggerFactory.getLogger("Rusty").warn("Server permissions fail closed", e);
    }
    var present = new HashSet<UUID>();
    for (ServerPlayer player : server.getPlayerList().getPlayers()) {
      UUID id = player.getUUID();
      present.add(id);
      Set<String> grants = new HashSet<>(rules.everyone);
      var personal = rules.players.get(id.toString());
      if (personal != null) grants.addAll(personal);
      String[] capabilities = {"view", "combat", "movement", "assistance"};
      int mask = 0;
      for (int i = 0; i < capabilities.length; i++)
        if (grants.contains(capabilities[i])) mask |= 1 << i;
      Delivery previous = SENT.get(id);
      if (previous == null || previous.player() != player || previous.mask() != mask) {
        if (send.test(player, new ConsentPayload(mask))) SENT.put(id, new Delivery(player, mask));
      }
    }
    SENT.keySet().retainAll(present);
  }
}
