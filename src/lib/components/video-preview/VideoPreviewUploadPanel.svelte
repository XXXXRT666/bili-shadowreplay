<script lang="ts">
  import { Pen } from "lucide-svelte";
  import TypeSelect from "../TypeSelect.svelte";
  import type { Profile, VideoItem } from "../../interface";

  export let video: VideoItem;
  export let title = "";
  export let tid = 0;
  export let uidSelected = 0;
  export let desc = "";
  export let tag = "";
  export let dynamic = "";
  export let accounts: Array<{ value: number; name: string }> = [];
  export let currentPostEventId: string | null = null;
  export let showCoverEditor = false;
  export let showDefaultCoverIcon = false;
  export let handleCoverError: ((event: Event) => void) | undefined = undefined;
  export let doPost: (() => void) | undefined = undefined;
  export let cancelPost: (() => void) | undefined = undefined;
</script>

<div class="p-4 space-y-6">
  {#if video && video.id != -1}
    <section>
      <div class="group">
        <div class="text-sm text-gray-400 mb-2 flex items-center justify-between">
          <span>视频封面</span>
          <button
            class="text-[#0A84FF] hover:text-[#0A84FF]/80 transition-colors duration-200 flex items-center space-x-1"
            on:click={() => (showCoverEditor = true)}
          >
            <Pen class="w-4 h-4" />
            <span class="text-xs">创建新封面</span>
          </button>
        </div>
        <div class="relative rounded-xl overflow-hidden bg-black/20 border border-gray-800/50">
          {#if video.cover && video.cover.trim() !== ""}
            <img
              src={video.cover}
              alt="视频封面"
              class="w-full"
              on:error={handleCoverError}
              style:display={showDefaultCoverIcon ? "none" : "block"}
            />
          {/if}
          {#if !video.cover || video.cover.trim() === "" || showDefaultCoverIcon}
            <div class="w-full aspect-video flex items-center justify-center bg-gray-800">
              <svg
                class="w-16 h-16 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
                ></path>
              </svg>
            </div>
          {/if}
        </div>
      </div>
    </section>
  {/if}

  <div class="space-y-4">
    <h3 class="text-sm font-medium text-gray-400">基本信息</h3>
    <div class="space-y-2">
      <label for="title" class="block text-sm font-medium text-gray-300">标题</label>
      <input
        id="title"
        type="text"
        bind:value={title}
        placeholder="输入视频标题"
        class="w-full px-3 py-2 bg-[#1c1c1e] text-white rounded-lg border border-gray-800/50 focus:border-[#0A84FF] transition duration-200 outline-none hover:border-gray-700/50"
      />
    </div>

    <div class="space-y-2">
      <label for="tid" class="block text-sm font-medium text-gray-300">视频分区</label>
      <div class="w-full" id="tid">
        <TypeSelect bind:value={tid} />
      </div>
    </div>

    <div class="space-y-2">
      <label for="uid" class="block text-sm font-medium text-gray-300">投稿账号</label>
      <select
        bind:value={uidSelected}
        class="w-full px-3 py-2 bg-[#1c1c1e] text-white rounded-lg border border-gray-800/50 focus:border-[#0A84FF] transition duration-200 outline-none appearance-none hover:border-gray-700/50"
      >
        <option value={0}>选择账号</option>
        {#each accounts as account}
          <option value={account.value}>{account.name}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="space-y-4">
    <h3 class="text-sm font-medium text-gray-400">详细信息</h3>
    <div class="space-y-2">
      <label for="desc" class="block text-sm font-medium text-gray-300">描述</label>
      <textarea
        id="desc"
        bind:value={desc}
        placeholder="输入视频描述"
        class="w-full px-3 py-2 bg-[#1c1c1e] text-white rounded-lg border border-gray-800/50 focus:border-[#0A84FF] transition duration-200 outline-none resize-none h-24 hover:border-gray-700/50"
      />
    </div>

    <div class="space-y-2">
      <label for="tag" class="block text-sm font-medium text-gray-300">标签</label>
      <input
        id="tag"
        type="text"
        bind:value={tag}
        placeholder="输入视频标签，用逗号分隔"
        class="w-full px-3 py-2 bg-[#1c1c1e] text-white rounded-lg border border-gray-800/50 focus:border-[#0A84FF] transition duration-200 outline-none hover:border-gray-700/50"
      />
    </div>

    <div class="space-y-2">
      <label for="dynamic" class="block text-sm font-medium text-gray-300">动态</label>
      <textarea
        id="dynamic"
        bind:value={dynamic}
        placeholder="输入动态内容"
        class="w-full px-3 py-2 bg-[#1c1c1e] text-white rounded-lg border border-gray-800/50 focus:border-[#0A84FF] transition duration-200 outline-none resize-none h-24 hover:border-gray-700/50"
      />
    </div>
  </div>

  {#if video}
    <div class="pt-4">
      <div class="flex gap-2">
        <button
          on:click={doPost}
          disabled={currentPostEventId != null || !uidSelected}
          class="flex-1 px-3 py-2 bg-[#0A84FF] text-white rounded-lg transition-all duration-200 hover:bg-[#0A84FF]/90 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center space-x-2 text-sm"
        >
          {#if currentPostEventId != null}
            <div class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
          {/if}
          <span id="post-prompt">投稿</span>
        </button>
        {#if currentPostEventId != null}
          <button
            on:click={cancelPost}
            class="px-3 py-2 bg-red-500 text-white rounded-lg transition-all duration-200 hover:bg-red-500/90 flex items-center justify-center text-sm"
          >
            取消
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
